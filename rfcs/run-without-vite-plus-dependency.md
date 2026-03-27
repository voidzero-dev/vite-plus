# RFC: `vp run` Without vite-plus Dependency

## Summary

Allow `vp run <script>` to work in projects that do not have `vite-plus` as a dependency. When vite-plus is not found in the nearest `package.json`'s `dependencies` or `devDependencies`, fall back to executing `<package-manager> run <script> [args...]` directly from the Rust layer, bypassing the JS delegation entirely.

## Motivation

Currently, all Category C commands (`vp run`, `vp build`, `vp test`, `vp lint`, `vp dev`, `vp fmt`, `vp preview`, `vp cache`) delegate to the local vite-plus CLI via the JS layer. When vite-plus is not installed as a dependency, `packages/global/src/local/bin.ts` prompts the user to add it to devDependencies:

```
Local "vite-plus" package was not found
? Do you want to add vite-plus to devDependencies? (Y/n)
```

If the user declines, it exits with:

```
Please add vite-plus to devDependencies first
```

This creates several issues:

1. **Barrier to adoption**: Users who install `vp` globally cannot use `vp run dev` as a drop-in replacement for `pnpm run dev` without first adding vite-plus to their project
2. **Unnecessary overhead**: The current flow downloads a Node.js runtime and enters the JS layer just to discover that vite-plus is missing
3. **Friction in existing projects**: Projects that want to use `vp` for its managed Node.js runtime and package manager features (install, add, remove) but not the task runner are blocked from using `vp run`

### Current Pain Points

```bash
# User installs vp globally and tries to use it
$ vp run dev
Local "vite-plus" package was not found
? Do you want to add vite-plus to devDependencies? (Y/n) n
Please add vite-plus to devDependencies first

# User just wants the equivalent of:
$ pnpm run dev
```

### Proposed Solution

```bash
# Without vite-plus as a dependency, falls back to PM run
$ vp run dev
# Executes: pnpm run dev (or npm/yarn depending on project)

# With vite-plus as a dependency, uses full task runner
$ vp run -r build
# Executes via vite-plus task runner with recursive + topological ordering
```

## Proposed Solution

### Detection Logic

Check the nearest `package.json` from the current working directory for `vite-plus` in `dependencies` or `devDependencies`. This matches the existing JS-side behavior in `hasVitePlusDependency(readNearestPackageJson(cwd))`.

**Decision: Check `package.json` only, NOT `node_modules`**

- If vite-plus is listed in `package.json` but not installed, the user must run `install` manually
- If vite-plus is NOT listed in `package.json`, we fall back to PM run
- Checking `node_modules` would be fragile (hoisted deps, workspaces) and inconsistent with the intent

### Scope

This RFC applies **only to `vp run`**. Other Category C commands (`build`, `test`, `lint`, `dev`, `fmt`, `preview`, `cache`) are vite-plus specific features and do not have natural PM fallbacks:

- `vp build` = Vite build (not `pnpm build`)
- `vp test` = Vitest (not `pnpm test`)
- `vp lint` = OxLint (not `pnpm lint`)

These should continue to delegate to the local CLI and prompt for vite-plus installation.

### Command Mapping

When falling back to PM run, all arguments are passed through as-is:

| `vp run` invocation      | Fallback command           | Notes                    |
| ------------------------ | -------------------------- | ------------------------ |
| `vp run dev`             | `pnpm run dev`             | Basic script execution   |
| `vp run dev --port 3000` | `pnpm run dev --port 3000` | Args passed through      |
| `vp run -r build`        | `pnpm run -r build`        | PM ignores unknown flags |
| `vp run app#build`       | `pnpm run app#build`       | PM treats as script name |

vite-plus specific flags (`-r`, `--recursive`, `--topological`, `package#task` syntax) are only meaningful when vite-plus is installed. When falling back, these are passed verbatim to the PM which will naturally error with "Missing script" -- this is correct behavior since these features require vite-plus.

### Architecture: Rust-Side Fallback

The fallback is implemented in the Rust layer, **before** entering the JS delegation flow. This avoids the unnecessary overhead of downloading Node.js runtime and entering the JS layer.

```
                  vp run <args>
                       |
               has_vite_plus_dependency(cwd)?
                   /        \
                 yes         no
                  |           |
          delegate to JS     build PM
          (existing flow)      |
                          <pm> run <args>
```

## Implementation Architecture

### 1. Dependency Check Utility

**File**: `crates/vite_global_cli/src/commands/mod.rs`

A utility function that walks up from `cwd` to find the nearest `package.json` and checks for vite-plus:

```rust
use std::collections::HashMap;
use std::io::BufReader;

#[derive(serde::Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct DepCheckPackageJson {
    #[serde(default)]
    dependencies: HashMap<String, serde_json::Value>,
    #[serde(default)]
    dev_dependencies: HashMap<String, serde_json::Value>,
}

/// Check if vite-plus is listed in the nearest package.json's
/// dependencies or devDependencies.
///
/// Returns `true` if vite-plus is found, `false` if not found
/// or if no package.json exists.
pub fn has_vite_plus_dependency(cwd: &AbsolutePath) -> bool {
    let mut current = cwd;
    loop {
        let package_json_path = current.join("package.json");
        if package_json_path.exists() {
            if let Ok(file) = std::fs::File::open(&package_json_path) {
                if let Ok(pkg) = serde_json::from_reader::<_, DepCheckPackageJson>(
                    BufReader::new(file)
                ) {
                    return pkg.dependencies.contains_key("vite-plus")
                        || pkg.dev_dependencies.contains_key("vite-plus");
                }
            }
            return false; // Found package.json but couldn't parse deps → treat as no dependency
        }
        match current.parent() {
            Some(parent) if parent != current => current = parent,
            _ => return false, // Reached filesystem root
        }
    }
}
```

### 2. PM Run Command

**File**: `crates/vite_install/src/commands/run.rs` (new file)

Following the established pattern from `dlx.rs`:

```rust
use std::{collections::HashMap, process::ExitStatus};
use vite_command::run_command;
use vite_error::Error;
use vite_path::AbsolutePath;
use crate::package_manager::{PackageManager, PackageManagerType, ResolveCommandResult, format_path_env};

impl PackageManager {
    /// Run `<pm> run <args>` to execute a package.json script.
    pub async fn run_script_command(
        &self,
        args: &[String],
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_run_script_command(args);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the `<pm> run <args>` command.
    #[must_use]
    pub fn resolve_run_script_command(&self, args: &[String]) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut cmd_args: Vec<String> = vec!["run".to_string()];
        cmd_args.extend(args.iter().cloned());

        let bin_path = match self.client {
            PackageManagerType::Pnpm => "pnpm",
            PackageManagerType::Npm => "npm",
            PackageManagerType::Yarn => "yarn",
        };

        ResolveCommandResult {
            bin_path: bin_path.to_string(),
            args: cmd_args,
            envs,
        }
    }
}
```

**Register**: Add `pub mod run;` to `crates/vite_install/src/commands/mod.rs`

### 3. Run-or-Delegate Orchestration

**File**: `crates/vite_global_cli/src/commands/run_or_delegate.rs` (new file)

```rust
//! Run command with fallback to package manager when vite-plus is not a dependency.

use std::process::ExitStatus;
use vite_path::AbsolutePathBuf;
use crate::error::Error;

/// Execute `vp run <args>`.
///
/// If vite-plus is a dependency, delegate to the local CLI.
/// If not, fall back to `<pm> run <args>`.
pub async fn execute(
    cwd: AbsolutePathBuf,
    args: &[String],
) -> Result<ExitStatus, Error> {
    if super::has_vite_plus_dependency(&cwd) {
        tracing::debug!("vite-plus is a dependency, delegating to local CLI");
        super::delegate::execute(cwd, "run", args).await
    } else {
        tracing::debug!("vite-plus is not a dependency, falling back to package manager run");
        super::prepend_js_runtime_to_path_env(&cwd).await?;
        let package_manager = super::build_package_manager(&cwd).await?;
        Ok(package_manager.run_script_command(args, &cwd).await?)
    }
}
```

### 4. CLI Dispatch Update

**File**: `crates/vite_global_cli/src/cli.rs`, line 1541

```rust
// Before:
Commands::Run { args } => commands::delegate::execute(cwd, "run", &args).await,

// After:
Commands::Run { args } => commands::run_or_delegate::execute(cwd, &args).await,
```

## Design Decisions

### 1. Rust-Side vs JS-Side Fallback

**Decision**: Implement the fallback in the Rust layer.

**Rationale**:

- **Performance**: Avoids downloading Node.js runtime and entering JS layer when unnecessary
- **Consistency**: Follows the same pattern as other PM commands (install, add, remove) which are Rust-native
- **Reuse**: Leverages existing `build_package_manager()` and `prepend_js_runtime_to_path_env()` utilities

**Alternative rejected**: Modifying `packages/global/src/local/bin.ts` to add a PM fallback would work but still require downloading Node.js first.

### 2. Check Nearest package.json Only

**Decision**: Walk up from `cwd` to find the nearest `package.json`, check only that file.

**Rationale**:

- Matches the existing JS-side behavior (`readNearestPackageJson(cwd)`)
- In a monorepo, a sub-package without vite-plus in its own package.json should still fall back even if the root has it -- the user is running from that package's context
- Simple, predictable behavior

**Alternative rejected**: Checking all ancestor package.json files up to workspace root would be more permissive but inconsistent with JS-side behavior.

### 3. Return `false` When No package.json Found

**Decision**: When no `package.json` exists at all, treat as "no vite-plus dependency" and fall back to PM run.

**Rationale**:

- `build_package_manager()` will fail with "No package.json found" which is already a clear error
- No special error handling needed
- Consistent with PM commands that also require package.json

### 4. Scope Limited to `vp run`

**Decision**: Only `vp run` gets the PM fallback. Other commands (`build`, `test`, `lint`, etc.) continue requiring vite-plus.

**Rationale**:

- `vp run <script>` maps naturally to `<pm> run <script>`
- `vp build` means "Vite build", not `<pm> run build` -- there's no meaningful fallback
- This keeps the behavior clear and predictable

### 5. Pass-Through All Arguments

**Decision**: All arguments after `run` are passed verbatim to the PM.

**Rationale**:

- Simple implementation with no argument rewriting
- vite-plus specific flags (`-r`, `package#task`) are meaningless without vite-plus
- PM will naturally error on unknown flags/scripts

## Error Handling

### No package.json Found

```bash
$ cd /tmp && vp run dev
No package.json found.
```

This comes from `build_package_manager()` which is the standard error for all PM commands.

### Script Not Found

```bash
$ vp run nonexistent
# Falls back to: pnpm run nonexistent
 ERR_PNPM_NO_SCRIPT  Missing script: nonexistent
```

Standard PM error, no special handling needed.

### No Package Manager Detected

When `package.json` exists but has no `packageManager` field and no lockfiles:

```bash
$ vp run dev
# build_package_manager prompts for PM selection (existing behavior)
```

## User Experience

### Without vite-plus Dependency

```bash
# package.json has scripts.dev but no vite-plus dependency
$ vp run dev
# Detects pnpm (from pnpm-lock.yaml)
# Executes: pnpm run dev
> my-app@1.0.0 dev
> vite
  VITE v6.0.0  ready in 200ms
```

### With vite-plus Dependency

```bash
# package.json has vite-plus in devDependencies
$ vp run -r build
# Delegates to local vite-plus CLI
# Uses task runner with recursive + topological ordering
  my-lib  build  done in 1.2s
  my-app  build  done in 2.3s
```

### Mixed Monorepo

```bash
# Root package.json has vite-plus, sub-package does not
/workspace$ vp run dev          # → delegates to vite-plus (found in root deps)
/workspace/legacy-pkg$ vp run dev  # → falls back to PM run (not in legacy-pkg's deps)
```

## Testing Strategy

### Unit Tests

**`has_vite_plus_dependency` tests:**

```rust
#[test]
fn test_has_vite_plus_in_dev_dependencies() {
    // package.json: { "devDependencies": { "vite-plus": "^1.0.0" } }
    // → returns true
}

#[test]
fn test_has_vite_plus_in_dependencies() {
    // package.json: { "dependencies": { "vite-plus": "^1.0.0" } }
    // → returns true
}

#[test]
fn test_no_vite_plus_dependency() {
    // package.json: { "devDependencies": { "vite": "^6.0.0" } }
    // → returns false
}

#[test]
fn test_no_package_json() {
    // Empty temp directory
    // → returns false
}

#[test]
fn test_nested_directory_walks_up() {
    // parent/package.json has vite-plus, cwd is parent/child/
    // → returns true
}
```

**`resolve_run_script_command` tests:**

```rust
#[test]
fn test_pnpm_run_script() {
    let pm = create_mock_package_manager(PackageManagerType::Pnpm, "10.0.0");
    let result = pm.resolve_run_script_command(&["dev".into()]);
    assert_eq!(result.bin_path, "pnpm");
    assert_eq!(result.args, vec!["run", "dev"]);
}

#[test]
fn test_npm_run_script_with_args() {
    let pm = create_mock_package_manager(PackageManagerType::Npm, "11.0.0");
    let result = pm.resolve_run_script_command(&["dev".into(), "--port".into(), "3000".into()]);
    assert_eq!(result.bin_path, "npm");
    assert_eq!(result.args, vec!["run", "dev", "--port", "3000"]);
}

#[test]
fn test_yarn_run_script() {
    let pm = create_mock_package_manager(PackageManagerType::Yarn, "4.0.0");
    let result = pm.resolve_run_script_command(&["build".into()]);
    assert_eq!(result.bin_path, "yarn");
    assert_eq!(result.args, vec!["run", "build"]);
}
```

## Backward Compatibility

- **Projects with vite-plus**: No change in behavior. The `has_vite_plus_dependency` check passes, and delegation proceeds as before.
- **Projects without vite-plus**: Previously errored or prompted for installation. Now works by falling back to PM run.
- **No breaking changes**: This is strictly additive behavior.

## Future Enhancements

### 1. Extend to Other Commands

If demand exists, other commands could gain PM fallbacks:

```bash
vp dev   → <pm> run dev   (when no vite-plus)
vp build → <pm> run build (when no vite-plus)
```

This would require a separate RFC and careful consideration of when `vp build` should mean "Vite build" vs "run the build script".

### 2. Informational Message

Optionally show a message when falling back:

```bash
$ vp run dev
(vite-plus not found, using pnpm run)
> my-app@1.0.0 dev
```

This could be controlled by a `--verbose` flag or shown only once.

## Files Changed

| File                                                     | Action | Description                                                          |
| -------------------------------------------------------- | ------ | -------------------------------------------------------------------- |
| `crates/vite_global_cli/src/commands/mod.rs`             | Modify | Add `has_vite_plus_dependency()` + register `run_or_delegate` module |
| `crates/vite_global_cli/src/commands/run_or_delegate.rs` | Create | Orchestration: check deps, delegate or fallback                      |
| `crates/vite_install/src/commands/mod.rs`                | Modify | Register `pub mod run;`                                              |
| `crates/vite_install/src/commands/run.rs`                | Create | PM `run` command resolution                                          |
| `crates/vite_global_cli/src/cli.rs`                      | Modify | Update dispatch at line 1541                                         |
