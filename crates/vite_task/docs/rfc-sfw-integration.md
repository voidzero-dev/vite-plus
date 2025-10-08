# RFC: Socket Security Framework (sfw) Integration for `vite install`

## Summary

Add optional Socket Security Framework (`sfw`) integration to `vite install` command, enabling users to wrap package installations with security monitoring via the `--sfw` flag.

## Motivation

Socket Security Framework (sfw) is a firewall for package management that provides security protection during software installation and build processes. Integrating sfw with vite-plus allows users to:

1. **Enhanced Security**: Monitor and potentially restrict package installations at runtime
2. **Supply Chain Protection**: Detect suspicious behavior during dependency installation
3. **Opt-in Approach**: Keep security features optional to avoid disrupting existing workflows
4. **Ecosystem Compatibility**: Align with industry-standard security practices

## Background: What is sfw?

Socket Security Framework (`sfw`) is a security tool that wraps package manager commands to provide:

- Real-time monitoring of package installation activities
- Detection of suspicious scripts and behaviors
- Protection against supply chain attacks
- Support for multiple package managers: npm, pnpm, yarn, pip, cargo, etc.

**Example Usage:**

```bash
# Traditional install
pnpm install lodash

# With sfw protection
sfw pnpm install lodash
```

Reference: [Socket Firewall Free](https://github.com/SocketDev/sfw-free)

## Design Principles

### 1. Opt-in Only

- **Rationale**: Security tooling should not be forced on users; it adds overhead and may break certain workflows
- **Implementation**: Require explicit `--sfw` flag to enable
- **Default Behavior**: No change to existing installation workflow

### 2. Transparent Wrapping

- **Rationale**: Users should get the same experience with minimal disruption
- **Implementation**: Prepend `sfw` to the package manager command
- **Exit Codes**: Preserve original command exit codes

### 3. Validation Required

- **Rationale**: Fail fast if sfw is not available rather than silently ignoring
- **Implementation**: Check for sfw binary existence before execution
- **Error Message**: Provide clear installation instructions

### 4. No Automatic Installation

- **Rationale**: Avoid implicit tool installation; keep vite-plus's scope focused
- **Implementation**: Users must install sfw separately
- **Documentation**: Provide installation guide in error messages

## Proposed CLI Interface

### Flag Addition

```rust
/// Install command.
/// It will be passed to the package manager's install command currently.
#[command(disable_help_flag = true, alias = "i")]
Install {
    /// Enable Socket Security Framework (sfw) protection
    #[arg(long)]
    sfw: bool,

    /// Arguments to pass to vite install
    #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
    args: Vec<String>,
}
```

### Usage Examples

```bash
# Standard install (no change)
vite install

# With sfw protection
vite install --sfw

# With additional arguments
vite install --sfw lodash express

# With npm flags
vite install --sfw --save-dev typescript
```

### Help Text

```bash
$ vite install --help
Install dependencies for the workspace

Usage: vite install [OPTIONS] [ARGS]...

Options:
      --sfw              Enable Socket Security Framework protection
  -h, --help            Print help

Arguments:
  [ARGS]...            Arguments to pass to the package manager
```

## Implementation Details

### 1. Update `Commands` Enum (lib.rs)

**File:** `crates/vite_task/src/lib.rs`

```rust
#[derive(Subcommand, Debug)]
pub enum Commands {
    // ... other commands ...

    /// Install command.
    /// It will be passed to the package manager's install command currently.
    #[command(disable_help_flag = true, alias = "i")]
    Install {
        /// Enable Socket Security Framework (sfw) protection
        #[arg(long)]
        sfw: bool,

        /// Arguments to pass to vite install
        #[arg(allow_hyphen_values = true, trailing_var_arg = true)]
        args: Vec<String>,
    },
}
```

**Update match arm:**

```rust
Commands::Install { args, sfw } => {
    install::InstallCommand::builder(cwd)
        .sfw(*sfw)
        .build()
        .execute(args)
        .await?
}
```

### 2. Update `InstallCommand` Structure (install.rs)

**File:** `crates/vite_task/src/install.rs`

```rust
pub struct InstallCommand {
    workspace_root: AbsolutePathBuf,
    ignore_replay: bool,
    sfw: bool,
}

pub struct InstallCommandBuilder {
    workspace_root: AbsolutePathBuf,
    ignore_replay: bool,
    sfw: bool,
}

impl InstallCommandBuilder {
    pub const fn new(workspace_root: AbsolutePathBuf) -> Self {
        Self {
            workspace_root,
            ignore_replay: false,
            sfw: false,
        }
    }

    pub const fn ignore_replay(mut self) -> Self {
        self.ignore_replay = true;
        self
    }

    pub const fn sfw(mut self, enable: bool) -> Self {
        self.sfw = enable;
        self
    }

    pub fn build(self) -> InstallCommand {
        InstallCommand {
            workspace_root: self.workspace_root,
            ignore_replay: self.ignore_replay,
            sfw: self.sfw,
        }
    }
}
```

### 3. Add SFW Validation Function

```rust
/// Check if sfw binary is available in PATH
fn check_sfw_available() -> Result<(), Error> {
    if which::which("sfw").is_err() {
        return Err(Error::SwfNotInstalled);
    }
    Ok(())
}
```

### 4. Update Execute Method

```rust
impl InstallCommand {
    pub async fn execute(self, args: &Vec<String>) -> Result<ExecutionSummary, Error> {
        // Validate sfw availability if enabled
        if self.sfw {
            check_sfw_available()?;
        }

        // ... existing package manager detection logic ...

        let mut workspace = Workspace::partial_load(self.workspace_root)?;
        let resolve_command = package_manager.resolve_command();

        // Wrap command with sfw if enabled
        let (bin_path, install_args) = if self.sfw {
            let mut sfw_args = vec!["sfw".to_string(), resolve_command.bin_path];
            sfw_args.push("install".to_string());
            sfw_args.extend(args.iter().cloned());

            ("sfw".to_string(), sfw_args)
        } else {
            let mut install_args = vec!["install".to_string()];
            install_args.extend(args.iter().cloned());

            (resolve_command.bin_path, install_args)
        };

        let resolved_task = ResolvedTask::resolve_from_builtin_with_command_result(
            &workspace,
            "install",
            install_args.iter().map(String::as_str),
            ResolveCommandResult {
                bin_path,
                envs: resolve_command.envs
            },
            self.ignore_replay,
        )?;

        // ... rest of execution logic ...
    }
}
```

### 5. Add Error Type (vite_error crate)

**File:** `crates/vite_error/src/lib.rs`

```rust
#[derive(Debug, Error)]
pub enum Error {
    // ... existing variants ...

    #[error(
        "Socket Security Framework (sfw) is not installed.\n\
         Install it via:\n  \
         npm install -g sfw\n\
         For more info: https://socket.dev/sfw"
    )]
    SwfNotInstalled,
}
```

### 6. Add Dependency

**File:** `crates/vite_task/Cargo.toml`

```toml
[dependencies]
# ... existing dependencies ...
which = "7.0.0" # For checking binary availability
```

## Command Execution Flow

```
vite install --sfw lodash
    │
    ├─> Parse CLI args (sfw=true, args=["lodash"])
    │
    ├─> Validate sfw availability
    │   └─> which::which("sfw") → Ok() or Error::SwfNotInstalled
    │
    ├─> Detect package manager (pnpm/npm/yarn)
    │
    ├─> Build command:
    │   WITHOUT --sfw: ["pnpm", "install", "lodash"]
    │   WITH --sfw:    ["sfw", "pnpm", "install", "lodash"]
    │
    ├─> Execute via ResolvedTask
    │   └─> Wrapped in task execution framework
    │
    └─> Return ExecutionSummary with exit status
```

## Alternative Designs Considered

### 1. Auto-detect sfw

**Rejected**: Too implicit; users should explicitly opt into security tooling

### 2. Auto-install sfw

**Rejected**: Out of scope; adds complexity and external dependencies

### 3. Configuration file option

**Future**: Consider adding `vite-task.json` config:

```json
{
  "install": {
    "sfw": true
  }
}
```

### 4. Environment variable (VITE_SFW=1)

**Future**: Could complement flag-based approach

## Testing Strategy

### Unit Tests

**File:** `crates/vite_task/src/install.rs`

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_install_command_builder_with_sfw() {
        let workspace_root = AbsolutePathBuf::new(PathBuf::from(
            if cfg!(windows) { "C:\\test\\workspace" } else { "/test/workspace" }
        )).unwrap();

        let command = InstallCommandBuilder::new(workspace_root.clone())
            .sfw(true)
            .build();

        assert_eq!(command.workspace_root, workspace_root);
        assert!(command.sfw);
    }

    #[test]
    fn test_install_command_builder_without_sfw() {
        let workspace_root = AbsolutePathBuf::new(PathBuf::from(
            if cfg!(windows) { "C:\\test\\workspace" } else { "/test/workspace" }
        )).unwrap();

        let command = InstallCommandBuilder::new(workspace_root.clone()).build();

        assert_eq!(command.workspace_root, workspace_root);
        assert!(!command.sfw);
    }

    #[test]
    fn test_check_sfw_available() {
        // This test depends on environment
        // Skip in CI unless sfw is pre-installed
        if std::env::var("CI").is_ok() {
            return;
        }

        // If sfw exists, should succeed
        // If not, should fail with SwfNotInstalled error
        let result = check_sfw_available();
        match which::which("sfw") {
            Ok(_) => assert!(result.is_ok()),
            Err(_) => assert!(matches!(result.unwrap_err(), Error::SwfNotInstalled)),
        }
    }
}
```

### Integration Tests

**File:** `crates/vite_task/tests/install_sfw_test.rs`

```rust
#[tokio::test]
#[ignore = "requires sfw to be installed"]
async fn test_install_with_sfw_flag() {
    let temp_dir = TempDir::new().unwrap();
    let workspace_root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

    // Create minimal package.json
    let package_json = r#"{
        "name": "test-sfw-package",
        "version": "1.0.0",
        "packageManager": "pnpm@10.15.0"
    }"#;
    fs::write(workspace_root.join("package.json"), package_json).unwrap();

    let command = InstallCommand::builder(workspace_root)
        .sfw(true)
        .build();

    let result = command.execute(&vec![]).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_install_with_sfw_flag_when_sfw_not_installed() {
    // Mock environment where sfw is not available
    let temp_dir = TempDir::new().unwrap();
    let workspace_root = AbsolutePathBuf::new(temp_dir.path().to_path_buf()).unwrap();

    let package_json = r#"{
        "name": "test-package",
        "version": "1.0.0",
        "packageManager": "pnpm@10.15.0"
    }"#;
    fs::write(workspace_root.join("package.json"), package_json).unwrap();

    let command = InstallCommand::builder(workspace_root)
        .sfw(true)
        .build();

    let result = command.execute(&vec![]).await;

    // Should fail if sfw not installed
    if which::which("sfw").is_err() {
        assert!(matches!(result.unwrap_err(), Error::SwfNotInstalled));
    }
}
```

### Manual Testing Checklist

- [ ] `vite install` without `--sfw` works as before
- [ ] `vite install --sfw` fails with clear error when sfw not installed
- [ ] `vite install --sfw` succeeds when sfw is installed
- [ ] `vite install --sfw lodash` passes arguments correctly
- [ ] `vite i --sfw` works with alias
- [ ] Exit codes are preserved from underlying command
- [ ] Works with different package managers (pnpm, npm, yarn)
- [ ] CI environment detection still works correctly
- [ ] Interactive package manager selection works with `--sfw`

## Documentation Requirements

### 1. README Update

Add section under installation/usage:

````markdown
## Security Features

### Socket Security Framework (sfw)

Vite-plus supports optional integration with [Socket Security Framework](https://socket.dev/sfw)
for enhanced security during package installation.

**Installation:**

```bash
npm install -g sfw
```
````

**Usage:**

```bash
# Enable sfw protection for installation
vite install --sfw

# With specific packages
vite install --sfw lodash express
```

**Note:** The `--sfw` flag is opt-in and requires sfw to be installed separately.

```
### 2. CLI Help Text

Ensure `--help` output clearly describes the `--sfw` flag (already covered in implementation).

### 3. Error Messages

The error message for missing sfw should be helpful:
```

Error: Socket Security Framework (sfw) is not installed.

Install it via:
npm install -g sfw

For more info: https://socket.dev/sfw

````
## Performance Considerations

### Overhead

- **Binary check**: One-time `which` lookup (~1-5ms)
- **Process spawn**: Minimal overhead; sfw wraps the existing command
- **Runtime**: Depends on sfw's monitoring overhead (typically < 5% based on sfw benchmarks)

### Optimization Opportunities

1. **Cache sfw availability**: Store result of `which::which("sfw")` to avoid repeated lookups
2. **Skip validation in CI**: If sfw explicitly requested, trust it's available (fail at runtime instead)

## Security Considerations

### Trust Model

- **User Responsibility**: Users must trust sfw and install it themselves
- **No Bundling**: We do not bundle or auto-install sfw (reduces supply chain risk)
- **Transparent Execution**: Command construction is visible in debug logs

### Failure Modes

- **sfw not found**: Fail fast with clear error (Error::SwfNotInstalled)
- **sfw fails**: Preserve exit code; let user see sfw's error output
- **Misconfiguration**: sfw's own error handling takes precedence

## Migration Path

### Phase 1: Initial Implementation (This RFC)
- Add `--sfw` flag support
- Basic validation and error handling
- Documentation

### Phase 2: Configuration Support (Future)
- Add `vite-task.json` config option
- Environment variable support (VITE_SFW=1)

### Phase 3: Enhanced Integration (Future)
- Custom sfw configuration path (`--sfw-config`)
- Integration with cache system (respect sfw's verdict)
- Telemetry integration

## Alternatives for Users

Users who want sfw protection without this feature can:

1. **Wrapper script**: Create a shell alias/function
   ```bash
   alias vite-install-safe="sfw vite install"
````

2. **CI integration**: Use sfw directly in CI workflows
   ```yaml
   - run: sfw pnpm install
   ```

3. **Package.json scripts**: Wrap in npm scripts
   ```json
   {
     "scripts": {
       "install:safe": "sfw pnpm install"
     }
   }
   ```

## Success Metrics

- [ ] No performance regression for non-sfw users (< 1ms overhead)
- [ ] Clear error messages with actionable steps
- [ ] Zero breaking changes to existing workflows
- [ ] Positive user feedback on security feature opt-in approach

## Open Questions

1. **Should we support custom sfw binary paths?**
   - Decision: Not in initial implementation; use PATH lookup only

2. **Should we validate sfw version compatibility?**
   - Decision: No; let sfw handle its own version requirements

3. **Should we add telemetry for --sfw usage?**
   - Decision: No telemetry in initial implementation; revisit in Phase 3

4. **Should auto-install (crates/vite_task/src/lib.rs:152) respect --sfw?**
   - Decision: No; auto-install is internal. User-facing `vite install --sfw` is sufficient

## Implementation Checklist

- [ ] Update `Commands` enum in `lib.rs`
- [ ] Add `sfw` field to `InstallCommand` and builder
- [ ] Implement `check_sfw_available()` function
- [ ] Update `execute()` method with sfw wrapping logic
- [ ] Add `Error::SwfNotInstalled` variant
- [ ] Add `which` dependency to `Cargo.toml`
- [ ] Write unit tests for builder and validation
- [ ] Write integration tests
- [ ] Update CLI help text
- [ ] Update README documentation
- [ ] Manual testing on all supported platforms
- [ ] Update CHANGELOG

## References

- [Socket Security Framework (sfw-free)](https://github.com/SocketDev/sfw-free)
- [Socket.dev Documentation](https://socket.dev/sfw)
- [Vite-plus Install Command](./crates/vite_task/src/install.rs)
