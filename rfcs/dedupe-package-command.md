# RFC: Vite+ Dedupe Package Command

## Summary

Add `vp dedupe` command that automatically adapts to the detected package manager (pnpm/npm/yarn/bun) for optimizing dependency trees by removing duplicate packages and upgrading older dependencies to newer compatible versions in the lockfile. This helps reduce redundancy and improve project efficiency.

## Motivation

Currently, developers must manually use package manager-specific commands to deduplicate dependencies:

```bash
pnpm dedupe
npm dedupe
yarn dedupe  # yarn@2+ only
```

This creates friction in dependency management workflows and requires remembering different syntaxes. A unified interface would:

1. **Simplify dependency optimization**: One command works across all package managers
2. **Auto-detection**: Automatically uses the correct package manager
3. **Consistency**: Same syntax regardless of underlying tool
4. **Integration**: Works seamlessly with existing Vite+ features

### Current Pain Points

```bash
# Developer needs to know which package manager is used
pnpm dedupe                    # pnpm project
npm dedupe                     # npm project
yarn dedupe                    # yarn@2+ project

# Different check modes
pnpm dedupe --check            # pnpm - check without modifying
npm dedupe --dry-run           # npm - check without modifying
yarn dedupe --check            # yarn@2+ - check without modifying
```

### Proposed Solution

```bash
# Works for all package managers
vp dedupe                    # Deduplicate dependencies

# Check mode (dry-run)
vp dedupe --check            # Check if deduplication would make changes
```

## Proposed Solution

### Command Syntax

#### Dedupe Command

```bash
vp dedupe [OPTIONS]
```

**Examples:**

```bash
# Basic deduplication
vp dedupe

# Check mode (preview changes without modifying)
vp dedupe --check
```

### Command Mapping

#### Dedupe Command Mapping

**pnpm references:**

- https://pnpm.io/cli/dedupe
- Performs an install removing older dependencies in the lockfile if a newer version can be used

**npm references:**

- https://docs.npmjs.com/cli/v11/commands/npm-dedupe
- Reduces duplication in the package tree by removing redundant packages

**yarn references:**

- https://yarnpkg.com/cli/dedupe (yarn@2+)
- Note: yarn@2+ has a dedicated `yarn dedupe` command with `--check` mode support

| Vite+ Flag  | pnpm          | npm          | yarn@2+       | bun | Description                  |
| ----------- | ------------- | ------------ | ------------- | --- | ---------------------------- |
| `vp dedupe` | `pnpm dedupe` | `npm dedupe` | `yarn dedupe` | N/A | Deduplicate dependencies     |
| `--check`   | `--check`     | `--dry-run`  | `--check`     | N/A | Check if changes would occur |

**Note**:

- pnpm uses `--check` for dry-run, npm uses `--dry-run`, yarn@2+ uses `--check`
- yarn@1 does not have dedupe command and is not supported
- bun does not currently support a dedupe command

### Dedupe Behavior Differences Across Package Managers

#### pnpm

**Dedupe behavior:**

- Scans the lockfile (`pnpm-lock.yaml`) for duplicate dependencies
- Upgrades older versions to newer compatible versions where possible
- Removes redundant entries in the lockfile
- Performs a fresh install with optimized dependencies
- `--check` flag previews changes without modifying files

**Exit codes:**

- 0: Success or no changes needed
- Non-zero: Changes would be made (when using `--check`)

#### npm

**Dedupe behavior:**

- Searches the local package tree (`node_modules`) for duplicate packages
- Attempts to simplify the structure by moving dependencies up the tree
- Removes duplicate packages where semver allows
- Modifies both `node_modules` and `package-lock.json`
- `--dry-run` shows what would be done without making changes

**Exit codes:**

- 0: Success
- Non-zero: Error occurred

#### yarn@2+ (Berry)

**Dedupe behavior:**

- Has a dedicated `yarn dedupe` command
- Scans the lockfile (`yarn.lock`) for duplicate dependencies
- Deduplicates packages by removing redundant entries
- `--check` flag previews changes without modifying files
- Uses Plug'n'Play or node_modules depending on configuration

**Exit codes:**

- 0: Success or no changes needed
- Non-zero: Changes would be made (when using `--check`)

**Note**: yarn@1 does not have a dedupe command and is not supported by Vite+

### Implementation Architecture

#### 1. Command Structure

**File**: `crates/vite_task/src/lib.rs`

Add new command variant:

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... existing commands

    /// Deduplicate dependencies by removing older versions
    #[command(disable_help_flag = true)]
    Dedupe {
        /// Check if deduplication would make changes (pnpm: --check, npm: --dry-run)
        #[arg(long)]
        check: bool,

        /// Arguments to pass to package manager
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}
```

#### 2. Package Manager Adapter

**File**: `crates/vite_package_manager/src/commands/dedupe.rs` (new file)

```rust
use std::{collections::HashMap, process::ExitStatus};

use vite_error::Error;
use vite_path::AbsolutePath;

use crate::package_manager::{
    PackageManager, PackageManagerType, ResolveCommandResult, format_path_env, run_command,
};

#[derive(Debug, Default)]
pub struct DedupeCommandOptions<'a> {
    pub check: bool,
    pub pass_through_args: Option<&'a [String]>,
}

impl PackageManager {
    /// Run the dedupe command with the package manager.
    #[must_use]
    pub async fn run_dedupe_command(
        &self,
        options: &DedupeCommandOptions<'_>,
        cwd: impl AsRef<AbsolutePath>,
    ) -> Result<ExitStatus, Error> {
        let resolve_command = self.resolve_dedupe_command(options);
        run_command(&resolve_command.bin_path, &resolve_command.args, &resolve_command.envs, cwd)
            .await
    }

    /// Resolve the dedupe command.
    #[must_use]
    pub fn resolve_dedupe_command(&self, options: &DedupeCommandOptions) -> ResolveCommandResult {
        let bin_name: String;
        let envs = HashMap::from([("PATH".to_string(), format_path_env(self.get_bin_prefix()))]);
        let mut args: Vec<String> = Vec::new();

        match self.client {
            PackageManagerType::Pnpm => {
                bin_name = "pnpm".into();
                args.push("dedupe".into());

                // pnpm uses --check for dry-run
                if options.check {
                    args.push("--check".into());
                }
            }
            PackageManagerType::Yarn => {
                bin_name = "yarn".into();
                args.push("dedupe".into());

                // yarn@2+ supports --check
                if options.check {
                    args.push("--check".into());
                }
            }
            PackageManagerType::Npm => {
                bin_name = "npm".into();
                args.push("dedupe".into());

                if options.check {
                    args.push("--dry-run".into());
                }
            }
        }

        // Add pass-through args
        if let Some(pass_through_args) = options.pass_through_args {
            args.extend_from_slice(pass_through_args);
        }

        ResolveCommandResult { bin_path: bin_name, args, envs }
    }
}
```

**File**: `crates/vite_package_manager/src/commands/mod.rs`

Update to include dedupe module:

```rust
pub mod add;
mod install;
pub mod remove;
pub mod update;
pub mod link;
pub mod unlink;
pub mod dedupe;  // Add this line
```

#### 3. Dedupe Command Implementation

**File**: `crates/vite_task/src/dedupe.rs` (new file)

```rust
use vite_error::Error;
use vite_path::AbsolutePathBuf;
use vite_package_manager::{
    PackageManager,
    commands::dedupe::DedupeCommandOptions,
};
use vite_workspace::Workspace;

pub struct DedupeCommand {
    workspace_root: AbsolutePathBuf,
}

impl DedupeCommand {
    pub fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self { workspace_root }
    }

    pub async fn execute(
        self,
        check: bool,
        extra_args: Vec<String>,
    ) -> Result<ExitStatus, Error> {
        let package_manager = PackageManager::builder(&self.workspace_root).build().await?;

        // Build dedupe command options
        let dedupe_options = DedupeCommandOptions {
            check,
            pass_through_args: if extra_args.is_empty() { None } else { Some(&extra_args) },
        };

        let exit_status = package_manager
            .run_dedupe_command(&dedupe_options, &self.workspace_root)
            .await?;

        if !exit_status.success() {
            if check {
                eprintln!("Deduplication would result in changes");
            }
            return Err(Error::CommandFailed {
                command: "dedupe".to_string(),
                exit_code: exit_status.code(),
            });
        }

        Ok(exit_status)
    }
}
```

## Design Decisions

### 1. No Caching

**Decision**: Do not cache dedupe operations.

**Rationale**:

- Dedupe modifies lockfiles and dependency trees
- Side effects make caching inappropriate
- Each execution should analyze current state
- Similar to how install/add/remove work

### 2. Simplified Flag Support

**Decision**: Only support `--check` flag for dry-run validation.

**Rationale**:

- Keeps the command simple and focused
- pnpm and yarn@2+ use `--check`, npm uses `--dry-run`
- Unified flag that maps to appropriate package manager flag
- Additional workspace/filtering flags add unnecessary complexity

### 3. yarn Support

**Decision**: Only support yarn@2+, not yarn@1.

**Rationale**:

- yarn@2+ has dedicated `yarn dedupe` command with `--check` support
- yarn@1 does not have a dedupe command (per official documentation)
- Simplifies implementation by not requiring version detection
- Aligns with official yarn documentation

### 4. Exit Code Handling

**Decision**: Return non-zero exit code when `--check` detects changes.

**Rationale**:

- Matches pnpm behavior
- Useful for CI/CD pipelines
- Can validate if deduplication is needed
- Standard practice for check/dry-run modes

## Error Handling

### No Package Manager Detected

```bash
$ vp dedupe
Error: No package manager detected
Please run one of:
  - vp install (to set up package manager)
  - Add packageManager field to package.json
```

### Check Mode Detects Changes

```bash
$ vp dedupe --check
Checking if deduplication would make changes...
Changes detected. Run 'vp dedupe' to apply.
Exit code: 1
```

### Unsupported Flag Warning

```bash
$ vp dedupe --filter app
Warning: --filter not supported by npm, use --workspace instead
Running: npm dedupe
```

## User Experience

### Success Output

```bash
$ vp dedupe
Detected package manager: pnpm@10.15.0
Running: pnpm dedupe

Packages: -15
-15
Progress: resolved 250, reused 235, downloaded 0, added 0, done

Dependencies optimized. Removed 15 duplicate packages.

Done in 3.2s
```

```bash
$ vp dedupe --check
Detected package manager: pnpm@10.15.0
Running: pnpm dedupe --check

Would deduplicate 8 packages:
  - lodash: 4.17.20 → 4.17.21 (3 occurrences)
  - react: 18.2.0 → 18.3.1 (2 occurrences)
  - typescript: 5.3.0 → 5.5.0 (3 occurrences)

Run 'vp dedupe' to apply these changes.
Exit code: 1
```

```bash
$ vp dedupe --check
Detected package manager: npm@11.0.0
Running: npm dedupe --dry-run

removed 12 packages
updated 5 packages

This was a dry run. No changes were made.

Done in 4.5s
```

### Yarn@2+ Output

```bash
$ vp dedupe
Detected package manager: yarn@4.0.0
Running: yarn dedupe

➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: Done in 1.2s

Done in 1.2s
```

```bash
$ vp dedupe --check
Detected package manager: yarn@4.0.0
Running: yarn dedupe --check

➤ YN0000: Found 5 packages with duplicates
➤ YN0000: Run 'yarn dedupe' to apply changes

Exit code: 1
```

### No Changes Needed

```bash
$ vp dedupe
Detected package manager: pnpm@10.15.0
Running: pnpm dedupe

Already up-to-date

Done in 0.8s
```

## Alternative Designs Considered

### Alternative 1: Error on Unsupported Flags

```bash
vp dedupe --filter app  # on npm
Error: --filter flag not supported by npm
```

**Rejected because**:

- Too strict, prevents usage
- Better to warn and continue
- Users might have wrapper scripts
- Graceful degradation is preferred

### Alternative 2: Auto-Translate All Flags

```bash
vp dedupe --filter app  # on npm
# Automatically translates to: npm dedupe --workspace app
```

**Rejected because**:

- Different semantics between --filter and --workspace
- pnpm's --filter supports patterns, npm's --workspace doesn't
- Could lead to unexpected behavior
- Better to warn and let user adjust

### Alternative 3: Separate Check Command

```bash
vp dedupe:check
vp dedupe:run
```

**Rejected because**:

- More commands to remember
- Flags are more idiomatic
- Matches native package manager APIs
- Less intuitive than `--check` flag

## Implementation Plan

### Phase 1: Core Functionality

1. Add `Dedupe` command variant to `Commands` enum
2. Create `dedupe.rs` module in both crates
3. Implement package manager command resolution
4. Add basic error handling

### Phase 2: Advanced Features

1. Implement check/dry-run mode
2. Add workspace filtering support
3. Implement npm's dependency type filtering
4. Handle yarn@2+ special case

### Phase 3: Testing

1. Unit tests for command resolution
2. Integration tests with mock package managers
3. Test check mode behavior
4. Test workspace operations

### Phase 4: Documentation

1. Update CLI documentation
2. Add examples to README
3. Document package manager compatibility
4. Add CI/CD usage examples

## Testing Strategy

### Test Package Manager Versions

- pnpm@9.x (WIP)
- pnpm@10.x
- yarn@4.x (yarn@2+)
- npm@10.x
- npm@11.x (WIP)
- bun@1.x (N/A - bun does not support dedupe)

### Unit Tests

```rust
#[test]
fn test_pnpm_dedupe_basic() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_dedupe_command(&DedupeCommandOptions {
        ..Default::default()
    });
    assert_eq!(args, vec!["dedupe"]);
}

#[test]
fn test_pnpm_dedupe_check() {
    let pm = PackageManager::mock(PackageManagerType::Pnpm);
    let args = pm.resolve_dedupe_command(&DedupeCommandOptions {
        check: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["dedupe", "--check"]);
}

#[test]
fn test_npm_dedupe_basic() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let args = pm.resolve_dedupe_command(&DedupeCommandOptions {
        ..Default::default()
    });
    assert_eq!(args, vec!["dedupe"]);
}

#[test]
fn test_npm_dedupe_check() {
    let pm = PackageManager::mock(PackageManagerType::Npm);
    let args = pm.resolve_dedupe_command(&DedupeCommandOptions {
        check: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["dedupe", "--dry-run"]);
}

#[test]
fn test_yarn_dedupe_basic() {
    let pm = PackageManager::mock(PackageManagerType::Yarn);
    let args = pm.resolve_dedupe_command(&DedupeCommandOptions {
        ..Default::default()
    });
    assert_eq!(args, vec!["dedupe"]);
}

#[test]
fn test_yarn_dedupe_check() {
    let pm = PackageManager::mock(PackageManagerType::Yarn);
    let args = pm.resolve_dedupe_command(&DedupeCommandOptions {
        check: true,
        ..Default::default()
    });
    assert_eq!(args, vec!["dedupe", "--check"]);
}
```

### Integration Tests

Create fixtures for testing with each package manager:

```
fixtures/dedupe-test/
  pnpm-workspace.yaml
  package.json
  packages/
    app/
      package.json (with duplicate deps)
    utils/
      package.json (with duplicate deps)
  test-steps.json
```

Test cases:

1. Basic deduplication
2. Check mode without modifying
3. Exit code verification for check mode
4. Pass-through arguments handling
5. Package manager detection and command mapping

## CLI Help Output

```bash
$ vp dedupe --help
Deduplicate dependencies by removing older versions

Usage: vp dedupe [OPTIONS] [-- <PASS_THROUGH_ARGS>...]

Options:
  --check                    Check if deduplication would make changes
                             (pnpm: --check, npm: --dry-run, yarn@2+: --check)

Behavior by Package Manager:
  pnpm:    Removes older dependencies from lockfile, upgrades to newer compatible versions
  npm:     Reduces duplication in package tree by moving dependencies up the tree
  yarn@2+: Scans lockfile and removes duplicate package entries

Note: yarn@1 does not have a dedupe command and is not supported

Examples:
  vp dedupe                          # Deduplicate all dependencies
  vp dedupe --check                  # Check if changes would occur
  vp dedupe -- --some-flag           # Pass custom flags to package manager
```

## Performance Considerations

1. **No Caching**: Operations run directly without cache overhead
2. **Lockfile Analysis**: Fast lockfile parsing and optimization
3. **Single Execution**: Unlike task runner, these are one-off operations
4. **Auto-Detection**: Reuses existing package manager detection (already cached)
5. **CI/CD Optimization**: Check mode enables quick validation without full install

## Security Considerations

1. **Lockfile Integrity**: Maintains lockfile integrity while optimizing
2. **Version Constraints**: Respects semver constraints from package.json
3. **No Unexpected Upgrades**: Only deduplicates within allowed version ranges
4. **Audit Compatibility**: Works with audit commands to ensure security

## Backward Compatibility

This is a new feature with no breaking changes:

- Existing commands unaffected
- New command is additive
- No changes to task configuration
- No changes to caching behavior

## Migration Path

### Adoption

Users can start using immediately:

```bash
# Old way
pnpm dedupe
npm dedupe

# New way (works with any package manager)
vp dedupe
```

### CI/CD Integration

```yaml
# Before
- run: pnpm dedupe --check

# After (works with any package manager)
- run: vp dedupe --check
```

## Real-World Usage Examples

### Local Development

```bash
# After installing many packages over time
vp dedupe                     # Clean up duplicates

# Check if cleanup is needed
vp dedupe --check             # Preview changes
```

### CI/CD Pipeline

```yaml
name: Check Dependency Optimization
on: [pull_request]

jobs:
  dedupe-check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: vp install
      - run: vp dedupe --check
        name: Verify dependencies are optimized
```

### Post-Update Workflow

```bash
# Update dependencies
vp update --latest

# Deduplicate after updates
vp dedupe

# Verify everything still works
vp test
```

## Package Manager Compatibility

| Feature       | pnpm         | npm            | yarn@2+      | bun              | Notes                                     |
| ------------- | ------------ | -------------- | ------------ | ---------------- | ----------------------------------------- |
| Basic dedupe  | ✅ `dedupe`  | ✅ `dedupe`    | ✅ `dedupe`  | ❌ Not supported | bun has no dedupe command                 |
| Check/Dry-run | ✅ `--check` | ✅ `--dry-run` | ✅ `--check` | ❌ Not supported | npm uses different flag name              |
| Exit codes    | ✅ Supported | ✅ Supported   | ✅ Supported | ❌ Not supported | All return non-zero on check with changes |

**Note**: yarn@1 does not have a dedupe command and is not supported. bun does not currently support a dedupe command.

## Future Enhancements

### 1. Dedupe Report

Generate detailed report of deduplication changes:

```bash
vp dedupe --report

# Output:
Deduplication Report:
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Package         Old Version    New Version    Occurrences
lodash          4.17.20        4.17.21        3
react           18.2.0         18.3.1         2
typescript      5.3.0          5.5.0          3
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Total: 8 packages deduplicated
```

### 2. Auto-Dedupe on Install

Automatically deduplicate after install:

```bash
vp install --auto-dedupe

# Or configure in vite-task.json
{
  "options": {
    "autoDedupe": true
  }
}
```

### 3. Dedupe Policy Checking

Enforce deduplication policies in CI:

```bash
vp dedupe --policy strict  # Fail if any duplicates exist
vp dedupe --policy warn    # Warn but don't fail
```

### 4. Dependency Analysis

Show why packages are duplicated:

```bash
vp dedupe --why lodash

# Output:
lodash@4.17.20:
  - Required by: package-a@1.0.0 (via ^4.17.0)
  - Required by: package-b@2.0.0 (via ~4.17.20)

lodash@4.17.21:
  - Required by: package-c@3.0.0 (via ^4.17.21)

Recommendation: All can use lodash@4.17.21
```

## Open Questions

1. **Should we auto-run dedupe after updates?**
   - Proposed: No, keep commands separate
   - Users can combine: `vp update && vp dedupe`
   - Later: Add `--auto-dedupe` flag to update command

2. **Should we show detailed diff in check mode?**
   - Proposed: Yes, show what would change
   - Helps users understand impact
   - Use package manager's native output

3. **Should we support force dedupe (ignore semver)?**
   - Proposed: No, too risky
   - Could break compatibility
   - Let package managers handle constraints

4. **Should we warn about security vulnerabilities during dedupe?**
   - Proposed: Later enhancement
   - Run audit after dedupe
   - Integrate with existing audit tools

5. **Should we support interactive mode?**
   - Proposed: Later enhancement
   - Let users choose which packages to dedupe
   - Similar to `vp update --interactive`

## Success Metrics

1. **Adoption**: % of users using `vp dedupe` vs direct package manager
2. **Dependency Reduction**: Average reduction in duplicate packages
3. **CI Integration**: Usage in CI/CD pipelines for validation
4. **Error Rate**: Track command failures vs package manager direct usage

## Conclusion

This RFC proposes adding `vp dedupe` command to provide a unified interface for dependency deduplication across pnpm/npm/yarn@2+/bun. The design:

- ✅ Automatically adapts to detected package manager
- ✅ Supports check mode for validation (maps to --check for pnpm/yarn@2+, --dry-run for npm)
- ✅ Simple, focused API with only essential --check flag
- ✅ yarn@2+ support with native dedupe command
- ✅ Pass-through args for advanced use cases
- ✅ No caching overhead
- ✅ Simple implementation leveraging existing infrastructure
- ✅ CI/CD friendly with exit codes
- ✅ Extensible for future enhancements

The implementation follows the same patterns as other package management commands while providing a simple, unified interface for dependency deduplication. By focusing only on the essential --check flag, the command remains easy to use and understand.
