# RFC: Vite+ dlx Command

## Summary

Add `vite dlx` command that fetches a package from the registry without installing it as a dependency, hotloads it, and runs whatever default command binary it exposes. This provides a unified interface across pnpm, npm, and yarn for executing remote packages temporarily.

## Motivation

Currently, developers must use package manager-specific commands for executing remote packages:

```bash
# pnpm
pnpm dlx create-react-app my-app
pnpm dlx typescript tsc --version

# npm
npx create-react-app my-app
npm exec -- create-react-app my-app

# yarn (v2+ only)
yarn dlx create-react-app my-app
```

This creates several issues:

1. **Cognitive Load**: Developers must remember different commands for each package manager
2. **Context Switching**: When working across projects with different package managers, developers need to switch mental models
3. **Script Portability**: Scripts that use dlx-like commands are tied to a specific package manager
4. **Yarn 1.x Incompatibility**: Yarn Classic doesn't have a `dlx` command at all, requiring fallback to `npx`

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm dlx create-vue my-app          # pnpm project
npx create-vue my-app               # npm project
yarn dlx create-vue my-app          # yarn@2+ project (doesn't work in yarn@1)

# Different syntax for specifying packages
pnpm --package=typescript dlx tsc --version
npm exec --package=typescript -- tsc --version
yarn dlx -p typescript tsc --version

# Shell mode has different flags
pnpm dlx -c 'echo "hello" | cowsay'
npm exec -c 'echo "hello" | cowsay'
yarn dlx -c 'echo "hello" | cowsay'  # Not supported in yarn
```

### Proposed Solution

```bash
# Works for all package managers
vite dlx create-vue my-app
vite dlx typescript tsc --version
vite dlx --package yo --package generator-webapp yo webapp
vite dlx -c 'echo "hello" | cowsay'
```

## Proposed Solution

### Command Syntax

```bash
vite dlx [OPTIONS] <package[@version]> [args...]
```

**Options:**

- `--package, -p <name>`: Specifies which package(s) to install before running the command. Can be specified multiple times.
- `--shell-mode, -c`: Executes the command within a shell environment (`/bin/sh` on UNIX, `cmd.exe` on Windows).
- `--silent, -s`: Suppresses all output except the executed command's output.
- `--yes, -y`: Automatically confirm any prompts (npm only).
- `--no, -n`: Automatically decline any prompts (npm only).

### Usage Examples

```bash
# Basic usage - run a package's default binary
vite dlx create-vue my-app

# Specify version
vite dlx create-vue@3.10.0 my-app
vite dlx typescript@5.5.4 tsc --version

# Separate package and command (when binary name differs from package name)
vite dlx --package @pnpm/meta-updater meta-updater --help

# Multiple packages
vite dlx --package yo --package generator-webapp yo webapp --skip-install

# Shell mode (pipe commands)
vite dlx --package cowsay --package lolcatjs -c 'echo "hi vite" | cowsay | lolcatjs'

# Silent mode
vite dlx -s create-vue my-app

# Combine options
vite dlx -p typescript -p @types/node -c 'tsc --init && node -e "console.log(123)"'
```

### Command Mapping

**References:**

- pnpm: https://pnpm.io/cli/dlx
- npm: https://docs.npmjs.com/cli/v10/commands/npm-exec
- yarn: https://yarnpkg.com/cli/dlx

| Vite+ Flag                      | pnpm               | npm                 | yarn@1      | yarn@2+          | Description                |
| ------------------------------- | ------------------ | ------------------- | ----------- | ---------------- | -------------------------- |
| `vite dlx <pkg>`                | `pnpm dlx <pkg>`   | `npm exec <pkg>`    | `npx <pkg>` | `yarn dlx <pkg>` | Execute package binary     |
| `--package <name>`, `-p <name>` | `--package <name>` | `--package=<name>`  | N/A         | `-p <name>`      | Specify package to install |
| `--shell-mode`, `-c`            | `-c`               | `-c`                | N/A         | N/A              | Execute in shell           |
| `--silent`, `-s`                | `--silent`         | `--loglevel silent` | `--quiet`   | `--quiet`        | Suppress output            |
| `--yes`, `-y`                   | N/A                | `--yes`             | N/A         | N/A              | Auto-confirm prompts       |
| `--no`, `-n`                    | N/A                | `--no`              | N/A         | N/A              | Auto-decline prompts       |

**Notes:**

- **yarn@1 (Classic)**: Does not have a native `dlx` command. Falls back to using `npx` which comes bundled with npm.
- **npm exec vs npx**: `npx` is essentially an alias for `npm exec --` with some convenience features. We use `npm exec` for consistency.
- **Shell mode**: Yarn 2+ does not support shell mode (`-c`), command will print a warning and try to execute anyway.
- **--package flag position**: For pnpm, `--package` comes before `dlx`. For npm, `--package` can be anywhere. For yarn, `-p` comes after `dlx`.

### Argument Handling

The `dlx` command has specific argument parsing requirements:

```bash
# Everything after the package spec is passed to the executed command
vite dlx typescript tsc --version --help

# This runs: tsc --version --help
# NOT: typescript with vite dlx options --version --help
```

**Implementation approach:**

1. Parse known vite dlx options (`--package`, `-c`, `-s`, `-y`, `-n`)
2. First non-option argument is the package spec (with optional @version)
3. All remaining arguments are passed through to the executed command

## Implementation Architecture

### 1. Command Structure

**File**: `crates/vite_command/src/lib.rs`

Add new command:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Execute a package binary without installing it as a dependency
    #[command(disable_help_flag = true)]
    Dlx {
        /// Package(s) to install before running the command
        /// Can be specified multiple times
        #[arg(long, short = 'p', value_name = "NAME")]
        package: Vec<String>,

        /// Execute the command within a shell environment
        #[arg(long = "shell-mode", short = 'c')]
        shell_mode: bool,

        /// Suppress all output except the executed command's output
        #[arg(long, short = 's')]
        silent: bool,

        /// Automatically confirm any prompts (npm only)
        #[arg(long, short = 'y')]
        yes: bool,

        /// Automatically decline any prompts (npm only)
        #[arg(long, short = 'n')]
        no: bool,

        /// Package to execute (with optional @version) and arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}
```

### 2. Package Manager Adapter

**File**: `crates/vite_install/src/commands/dlx.rs` (new file)

```rust
use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

/// Options for the dlx command
pub struct DlxCommandOptions<'a> {
    /// Additional packages to install
    pub packages: &'a [String],
    /// The package to execute (first positional arg)
    pub package_spec: &'a str,
    /// Arguments to pass to the executed command
    pub args: &'a [String],
    /// Execute in shell mode
    pub shell_mode: bool,
    /// Suppress output
    pub silent: bool,
    /// Auto-confirm prompts (npm)
    pub yes: bool,
    /// Auto-decline prompts (npm)
    pub no: bool,
}

impl PackageManager {
    /// Resolve the dlx command for the detected package manager
    #[must_use]
    pub fn resolve_dlx_command(&self, options: &DlxCommandOptions) -> ResolveCommandResult {
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);

        match self.client {
            PackageManagerType::Pnpm => self.resolve_pnpm_dlx(options, envs),
            PackageManagerType::Npm => self.resolve_npm_dlx(options, envs),
            PackageManagerType::Yarn => {
                if self.version.starts_with("1.") {
                    // Yarn 1.x doesn't have dlx, fall back to npx
                    self.resolve_npx_fallback(options, envs)
                } else {
                    self.resolve_yarn_dlx(options, envs)
                }
            }
        }
    }

    fn resolve_pnpm_dlx(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        let mut args = Vec::new();

        // Add --package flags before dlx
        for pkg in options.packages {
            args.push("--package".into());
            args.push(pkg.clone());
        }

        args.push("dlx".into());

        // Add shell mode flag
        if options.shell_mode {
            args.push("-c".into());
        }

        // Add silent flag
        if options.silent {
            args.push("--silent".into());
        }

        // Add package spec
        args.push(options.package_spec.into());

        // Add command arguments
        args.extend(options.args.iter().cloned());

        ResolveCommandResult {
            bin_path: "pnpm".into(),
            args,
            envs,
        }
    }

    fn resolve_npm_dlx(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        let mut args = vec!["exec".into()];

        // Add package flags
        for pkg in options.packages {
            args.push(format!("--package={}", pkg));
        }

        // Add the main package as well
        if !options.packages.is_empty() || options.package_spec.contains('@') {
            args.push(format!("--package={}", options.package_spec));
        }

        // Add shell mode flag
        if options.shell_mode {
            args.push("-c".into());
        }

        // Add yes/no flags
        if options.yes {
            args.push("--yes".into());
        }
        if options.no {
            args.push("--no".into());
        }

        // Add silent flag
        if options.silent {
            args.push("--loglevel".into());
            args.push("silent".into());
        }

        // Add separator and command
        args.push("--".into());

        // For npm exec, we need to extract the command name from package spec
        let command = if options.packages.is_empty() {
            extract_command_from_spec(options.package_spec)
        } else {
            options.package_spec.to_string()
        };
        args.push(command);

        // Add command arguments
        args.extend(options.args.iter().cloned());

        ResolveCommandResult {
            bin_path: "npm".into(),
            args,
            envs,
        }
    }

    fn resolve_yarn_dlx(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        let mut args = vec!["dlx".into()];

        // Add package flags
        for pkg in options.packages {
            args.push("-p".into());
            args.push(pkg.clone());
        }

        // Add quiet flag for silent mode
        if options.silent {
            args.push("--quiet".into());
        }

        // Warn about unsupported shell mode
        if options.shell_mode {
            eprintln!("Warning: yarn dlx does not support shell mode (-c)");
        }

        // Add package spec
        args.push(options.package_spec.into());

        // Add command arguments
        args.extend(options.args.iter().cloned());

        ResolveCommandResult {
            bin_path: "yarn".into(),
            args,
            envs,
        }
    }

    fn resolve_npx_fallback(
        &self,
        options: &DlxCommandOptions,
        envs: HashMap<String, String>,
    ) -> ResolveCommandResult {
        eprintln!("Note: yarn@1 does not have dlx command, falling back to npx");

        let mut args = Vec::new();

        // Add package flags
        for pkg in options.packages {
            args.push("--package".into());
            args.push(pkg.clone());
        }

        // Add shell mode flag
        if options.shell_mode {
            args.push("-c".into());
        }

        // Add quiet flag for silent mode
        if options.silent {
            args.push("--quiet".into());
        }

        // Add yes/no flags
        if options.yes {
            args.push("--yes".into());
        }
        if options.no {
            args.push("--no".into());
        }

        // Add package spec
        args.push(options.package_spec.into());

        // Add command arguments
        args.extend(options.args.iter().cloned());

        ResolveCommandResult {
            bin_path: "npx".into(),
            args,
            envs,
        }
    }

    /// Run the dlx command
    pub async fn run_dlx_command(
        &self,
        options: &DlxCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_dlx_command(options);
        run_command(
            &resolve_command.bin_path,
            &resolve_command.args,
            &resolve_command.envs,
            cwd,
        )
        .await
    }
}

/// Extract command name from package spec
/// e.g., "create-vue@3.10.0" -> "create-vue"
fn extract_command_from_spec(spec: &str) -> String {
    // Handle scoped packages: @scope/pkg@version -> pkg
    if spec.starts_with('@') {
        // Find the second @ (version separator) or use the whole thing
        if let Some(slash_pos) = spec.find('/') {
            let after_slash = &spec[slash_pos + 1..];
            if let Some(at_pos) = after_slash.find('@') {
                return after_slash[..at_pos].to_string();
            }
            return after_slash.to_string();
        }
    }

    // Non-scoped: pkg@version -> pkg
    if let Some(at_pos) = spec.find('@') {
        return spec[..at_pos].to_string();
    }

    spec.to_string()
}
```

### 3. Command Handler

**File**: `crates/vite_task/src/dlx.rs` (new file)

```rust
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_install::commands::dlx::DlxCommandOptions;
use vite_install::PackageManager;

pub struct DlxCommand {
    cwd: AbsolutePathBuf,
}

impl DlxCommand {
    pub fn new(cwd: AbsolutePathBuf) -> Self {
        Self { cwd }
    }

    pub async fn execute(
        self,
        packages: Vec<String>,
        shell_mode: bool,
        silent: bool,
        yes: bool,
        no: bool,
        args: Vec<String>,
    ) -> Result<i32, Error> {
        if args.is_empty() {
            return Err(Error::InvalidArgument(
                "dlx requires a package name".to_string(),
            ));
        }

        // First arg is the package spec, rest are command args
        let package_spec = &args[0];
        let command_args = &args[1..];

        let package_manager = PackageManager::builder(&self.cwd).build().await?;

        let options = DlxCommandOptions {
            packages: &packages,
            package_spec,
            args: command_args,
            shell_mode,
            silent,
            yes,
            no,
        };

        let exit_status = package_manager.run_dlx_command(&options, &self.cwd).await?;

        Ok(exit_status.code().unwrap_or(1))
    }
}
```

## Design Decisions

### 1. Fallback to npx for Yarn 1.x

**Decision**: When using yarn@1, fall back to `npx` instead of failing.

**Rationale**:

- Yarn Classic doesn't have a `dlx` command
- `npx` comes bundled with npm and is almost always available
- Provides a working solution rather than an error
- Users are informed via a note that fallback is being used

### 2. Package Flag Position

**Decision**: Accept `--package` flags anywhere before the package spec.

**Rationale**:

- pnpm requires `--package` before `dlx`
- npm allows `--package` anywhere
- yarn requires `-p` after `dlx`
- Our unified interface accepts it anywhere and maps accordingly

### 3. Shell Mode Warning for Yarn

**Decision**: Warn but proceed when shell mode is used with yarn.

**Rationale**:

- Yarn 2+ doesn't support shell mode
- Better to warn and try than to fail entirely
- Users can see the warning and adjust if needed
- Some commands might work without shell mode

### 4. Silent Mode Mapping

**Decision**: Map `--silent` to equivalent flags for each PM.

**Rationale**:

- pnpm uses `--silent`
- npm uses `--loglevel silent`
- yarn uses `--quiet`
- Provides consistent UX across package managers

### 5. Command Extraction from Package Spec

**Decision**: Automatically extract command name from package spec for npm.

**Rationale**:

- `npm exec` requires explicit command name after `--`
- `pnpm dlx` and `yarn dlx` infer command from package
- Automation provides consistent UX
- Handles scoped packages correctly

## Error Handling

### Missing Package Spec

```bash
$ vite dlx
Error: dlx requires a package name

Usage: vite dlx [OPTIONS] <package[@version]> [args...]

Examples:
  vite dlx create-vue my-app
  vite dlx typescript tsc --version
```

### Package Not Found

```bash
$ vite dlx non-existent-package-xyz
Detected package manager: pnpm@10.15.0
Running: pnpm dlx non-existent-package-xyz
 ERR_PNPM_NO_IMPORTER_MANIFEST_FOUND  No package.json was found for "non-existent-package-xyz"
Exit code: 1
```

### Network Error

```bash
$ vite dlx create-vue my-app
Detected package manager: npm@11.0.0
Running: npm exec create-vue -- my-app
npm error code ENOTFOUND
npm error network request to https://registry.npmjs.org/create-vue failed
Exit code: 1
```

## User Experience

### Basic Execution

```bash
$ vite dlx create-vue my-app
Detected package manager: pnpm@10.15.0
Running: pnpm dlx create-vue my-app

Vue.js - The Progressive JavaScript Framework

✔ Project name: my-app
✔ Add TypeScript? Yes
...
```

### Version Specific

```bash
$ vite dlx typescript@5.5.4 tsc --version
Detected package manager: pnpm@10.15.0
Running: pnpm dlx typescript@5.5.4 tsc --version
Version 5.5.4
```

### Multiple Packages

```bash
$ vite dlx --package yo --package generator-webapp yo webapp
Detected package manager: npm@11.0.0
Running: npm exec --package=yo --package=generator-webapp -- yo webapp
? What would you like to do? Create a new webapp
...
```

### Shell Mode

```bash
$ vite dlx --package cowsay --package lolcatjs -c 'echo "Hello Vite+" | cowsay | lolcatjs'
Detected package manager: pnpm@10.15.0
Running: pnpm --package cowsay --package lolcatjs dlx -c 'echo "Hello Vite+" | cowsay | lolcatjs'
 _______________
< Hello Vite+  >
 ---------------
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

### Yarn 1.x Fallback

```bash
$ vite dlx create-vue my-app
Detected package manager: yarn@1.22.19
Note: yarn@1 does not have dlx command, falling back to npx
Running: npx create-vue my-app
...
```

## Alternative Designs Considered

### Alternative 1: Always Use npx

```bash
# Simply wrap npx for all package managers
vite dlx → npx
```

**Rejected because**:

- Loses integration with pnpm's store and caching
- Doesn't respect yarn 2+ project settings
- Inconsistent with other vite commands that use detected PM
- npx may not be available (though rare)

### Alternative 2: Top-Level Aliases

```bash
vite create-vue my-app    # Implicit dlx
vite x create-vue my-app  # Short alias
```

**Rejected because**:

- Conflicts with potential future commands
- Less explicit about what's happening
- Harder to discover and document
- Deviates from pnpm/npm/yarn conventions

### Alternative 3: No Fallback for Yarn 1.x

```bash
$ vite dlx create-vue
Error: yarn@1.22.19 does not support dlx command
```

**Rejected because**:

- Frustrating user experience
- npx fallback works well and is available
- Other tools (like bunx) also provide fallbacks
- Users shouldn't need to switch package managers for dlx

## Implementation Plan

### Phase 1: Core Infrastructure

1. Add `Dlx` variant to `Commands` enum in `vite_command`
2. Create `DlxCommandOptions` struct
3. Implement `resolve_dlx_command` for each package manager
4. Add `run_dlx_command` execution method

### Phase 2: Package Manager Support

1. Implement pnpm dlx resolution
2. Implement npm exec resolution
3. Implement yarn dlx resolution (v2+)
4. Implement npx fallback for yarn v1

### Phase 3: Testing

1. Unit tests for command resolution
2. Test package spec parsing
3. Test option mapping for each PM
4. Integration tests with mock package managers
5. Test yarn v1 fallback behavior

### Phase 4: Documentation

1. Update CLI help text
2. Add usage examples
3. Document package manager compatibility
4. Add troubleshooting guide

## Testing Strategy

### Unit Tests

```rust
#[test]
fn test_pnpm_dlx_basic() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = DlxCommandOptions {
        packages: &[],
        package_spec: "create-vue",
        args: &["my-app".into()],
        shell_mode: false,
        silent: false,
        yes: false,
        no: false,
    };
    let result = pm.resolve_dlx_command(&options);
    assert_eq!(result.bin_path, "pnpm");
    assert_eq!(result.args, vec!["dlx", "create-vue", "my-app"]);
}

#[test]
fn test_pnpm_dlx_with_packages() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = DlxCommandOptions {
        packages: &["yo".into(), "generator-webapp".into()],
        package_spec: "yo",
        args: &["webapp".into()],
        shell_mode: false,
        silent: false,
        yes: false,
        no: false,
    };
    let result = pm.resolve_dlx_command(&options);
    assert_eq!(
        result.args,
        vec!["--package", "yo", "--package", "generator-webapp", "dlx", "yo", "webapp"]
    );
}

#[test]
fn test_npm_exec_basic() {
    let pm = PackageManager::mock(PackageManagerType::Npm, "11.0.0");
    let options = DlxCommandOptions {
        packages: &[],
        package_spec: "create-vue",
        args: &["my-app".into()],
        shell_mode: false,
        silent: false,
        yes: false,
        no: false,
    };
    let result = pm.resolve_dlx_command(&options);
    assert_eq!(result.bin_path, "npm");
    assert_eq!(result.args, vec!["exec", "--", "create-vue", "my-app"]);
}

#[test]
fn test_yarn_v1_fallback_to_npx() {
    let pm = PackageManager::mock(PackageManagerType::Yarn, "1.22.19");
    let options = DlxCommandOptions {
        packages: &[],
        package_spec: "create-vue",
        args: &["my-app".into()],
        shell_mode: false,
        silent: false,
        yes: false,
        no: false,
    };
    let result = pm.resolve_dlx_command(&options);
    assert_eq!(result.bin_path, "npx");
    assert_eq!(result.args, vec!["create-vue", "my-app"]);
}

#[test]
fn test_yarn_v2_dlx() {
    let pm = PackageManager::mock(PackageManagerType::Yarn, "4.0.0");
    let options = DlxCommandOptions {
        packages: &[],
        package_spec: "create-vue",
        args: &["my-app".into()],
        shell_mode: false,
        silent: false,
        yes: false,
        no: false,
    };
    let result = pm.resolve_dlx_command(&options);
    assert_eq!(result.bin_path, "yarn");
    assert_eq!(result.args, vec!["dlx", "create-vue", "my-app"]);
}

#[test]
fn test_extract_command_from_spec() {
    assert_eq!(extract_command_from_spec("create-vue"), "create-vue");
    assert_eq!(extract_command_from_spec("create-vue@3.10.0"), "create-vue");
    assert_eq!(extract_command_from_spec("@vue/cli"), "cli");
    assert_eq!(extract_command_from_spec("@vue/cli@5.0.0"), "cli");
}

#[test]
fn test_shell_mode() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm, "10.0.0");
    let options = DlxCommandOptions {
        packages: &["cowsay".into()],
        package_spec: "echo hello | cowsay",
        args: &[],
        shell_mode: true,
        silent: false,
        yes: false,
        no: false,
    };
    let result = pm.resolve_dlx_command(&options);
    assert!(result.args.contains(&"-c".to_string()));
}
```

## CLI Help Output

```bash
$ vite dlx --help
Execute a package binary without installing it as a dependency

Usage: vite dlx [OPTIONS] <package[@version]> [args...]

Arguments:
  <package[@version]>  Package to execute (with optional version)
  [args...]            Arguments to pass to the executed command

Options:
  -p, --package <NAME>  Package(s) to install before running (can be used multiple times)
  -c, --shell-mode      Execute the command within a shell environment
  -s, --silent          Suppress all output except the executed command's output
  -y, --yes             Automatically confirm any prompts (npm only)
  -n, --no              Automatically decline any prompts (npm only)
  -h, --help            Print help

Examples:
  vite dlx create-vue my-app                              # Create a new Vue project
  vite dlx typescript@5.5.4 tsc --version                 # Run specific version
  vite dlx -p yo -p generator-webapp yo webapp            # Multiple packages
  vite dlx -c 'echo "hello" | cowsay'                     # Shell mode
  vite dlx -s create-vue my-app                           # Silent mode
```

## Package Manager Compatibility

| Feature           | pnpm    | npm     | yarn@1 | yarn@2+ | Notes                    |
| ----------------- | ------- | ------- | ------ | ------- | ------------------------ |
| Basic execution   | ✅ Full | ✅ Full | ⚠️ npx  | ✅ Full | yarn@1 uses npx fallback |
| Version specifier | ✅ Full | ✅ Full | ⚠️ npx  | ✅ Full |                          |
| --package flag    | ✅ Full | ✅ Full | ⚠️ npx  | ✅ Full |                          |
| Shell mode (-c)   | ✅ Full | ✅ Full | ⚠️ npx  | ❌ N/A  | yarn@2+ doesn't support  |
| Silent mode       | ✅ Full | ✅ Full | ⚠️ npx  | ✅ Full |                          |
| --yes/--no        | ❌ N/A  | ✅ Full | ⚠️ npx  | ❌ N/A  | npm-specific             |

## Security Considerations

1. **Remote Code Execution**: `dlx` inherently executes remote code. Users should:
   - Verify package names before execution
   - Use version specifiers for reproducibility
   - Review package contents when uncertain

2. **No Permanent Installation**: Packages are installed to a temporary cache, not project dependencies.
   - Reduces supply chain attack surface
   - No changes to package.json or lockfiles

3. **Shell Mode Risks**: Shell mode (`-c`) allows arbitrary shell commands.
   - Use with caution in scripts
   - Avoid interpolating untrusted input

4. **Build Scripts**: pnpm's `--allow-build` controls postinstall scripts.
   - By default, dlx packages can run build scripts
   - Consider security implications for untrusted packages

## Backward Compatibility

This is a new feature with no breaking changes:

- Existing commands unaffected
- New command is purely additive
- No changes to configuration format
- No changes to caching behavior

## Future Enhancements

### 1. Cache Management

```bash
vite dlx --clear-cache                # Clear dlx cache
vite dlx --cache-dir                  # Show cache location
```

### 2. Offline Mode

```bash
vite dlx --offline create-vue my-app  # Use cached version only
```

### 3. Registry Override

```bash
vite dlx --registry https://custom.registry.com create-vue my-app
```

### 4. Trust Configuration

```bash
# In vite-task.json
{
  "dlx": {
    "trustedPackages": ["create-vue", "typescript"],
    "allowBuild": false
  }
}
```

### 5. Execution History

```bash
vite dlx --history                    # Show recent dlx executions
vite dlx --replay 3                   # Re-run 3rd most recent command
```

## Real-World Usage Examples

### Project Scaffolding

```bash
# Create new projects with various frameworks
vite dlx create-vue my-vue-app
vite dlx create-react-app my-react-app
vite dlx create-next-app my-next-app
vite dlx create-svelte my-svelte-app
vite dlx @angular/cli ng new my-angular-app
```

### One-off Tools

```bash
# Format JSON
vite dlx prettier --write package.json

# Check TypeScript
vite dlx typescript tsc --noEmit

# Run ESLint
vite dlx eslint src/

# Generate licenses
vite dlx license-checker --json
```

### CI/CD Pipelines

```yaml
# GitHub Actions
- name: Create release notes
  run: vite dlx -s conventional-changelog-cli -p angular > CHANGELOG.md

- name: Check for vulnerabilities
  run: vite dlx snyk test

- name: Publish to npm
  run: vite dlx np --no-tests
```

### Development Utilities

```bash
# Quick HTTP server
vite dlx serve dist/

# JSON server for mocking
vite dlx json-server db.json

# Bundle analyzer
vite dlx source-map-explorer dist/*.js

# Dependency visualization
vite dlx madge --image deps.svg src/
```

## Conclusion

This RFC proposes adding `vite dlx` command to provide unified remote package execution across pnpm/npm/yarn. The design:

- ✅ Unified interface for all package managers
- ✅ Intelligent fallback for yarn@1
- ✅ Pass-through for advanced options
- ✅ Shell mode for complex commands
- ✅ Silent mode for CI/scripting
- ✅ Version specifiers for reproducibility
- ✅ Multiple package support
- ✅ Follows existing pnpm dlx conventions
- ✅ Simple implementation leveraging existing infrastructure

The command provides the convenience of `npx`/`pnpm dlx`/`yarn dlx` with automatic package manager detection, ensuring consistent developer experience regardless of the project's package manager choice.
