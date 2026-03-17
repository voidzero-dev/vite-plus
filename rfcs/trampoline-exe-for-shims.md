# RFC: Windows Trampoline `.exe` for Shims

## Status

Proposed

## Summary

Replace Windows `.cmd` wrapper scripts with lightweight trampoline `.exe` binaries for all shim tools (`vp`, `node`, `npm`, `npx`, `vpx`, and globally installed package binaries). This eliminates the `Terminate batch job (Y/N)?` prompt that appears when users press Ctrl+C, providing the same clean signal behavior as direct `.exe` invocation.

## Motivation

### The Problem

On Windows, the vite-plus CLI exposes tools through `.cmd` batch file wrappers:

```
~/.vite-plus/bin/
├── vp.cmd          → calls current\bin\vp.exe
├── node.cmd        → calls vp.exe env exec node
├── npm.cmd         → calls vp.exe env exec npm
├── npx.cmd         → calls vp.exe env exec npx
└── ...
```

When a user presses Ctrl+C while a command is running through a `.cmd` wrapper, `cmd.exe` intercepts the signal and displays:

```
Terminate batch job (Y/N)?
```

This is a fundamental limitation of batch file execution on Windows. The prompt:

- Interrupts the normal Ctrl+C workflow that users expect
- May appear multiple times (once per `.cmd` in the chain)
- Differs from Unix behavior where Ctrl+C cleanly terminates the process
- Cannot be suppressed from within the batch file

### Confirmed Behavior

As demonstrated in [issue #835](https://github.com/voidzero-dev/vite-plus/issues/835):

1. Running `vp dev` (through `vp.cmd`) shows `Terminate batch job (Y/N)?` on Ctrl+C
2. Running `~/.vite-plus/current/bin/vp.exe dev` directly does **NOT** show the prompt
3. Running `npm.cmd run dev` shows the prompt; running `npm.ps1 run dev` does not
4. The prompt can appear multiple times when `.cmd` wrappers chain (e.g., `vp.cmd` → `npm.cmd`)

### Why `.ps1` Scripts Are Not Sufficient

PowerShell `.ps1` scripts avoid the Ctrl+C issue but have critical limitations:

- `where.exe` and `which` do not discover `.ps1` files as executables
- Only work in PowerShell, not in `cmd.exe`, Git Bash, or other shells
- Cannot serve as universal shims

## Current Architecture

### Unix (Symlink-Based)

On Unix, shims are symlinks to the `vp` binary. The binary detects the tool name from `argv[0]`:

```
~/.vite-plus/bin/
├── vp   → ../current/bin/vp     (symlink)
├── node → ../current/bin/vp     (symlink)
├── npm  → ../current/bin/vp     (symlink)
└── npx  → ../current/bin/vp     (symlink)
```

When invoked as `node`, `argv[0]` is `"node"`, and the `vp` binary dispatches to shim mode. This is efficient (zero overhead) and survives `vp` binary updates (symlinks follow `current/` which is re-pointed on upgrade).

**Relevant code**: `crates/vite_global_cli/src/shim/mod.rs:detect_shim_tool()`

### Windows (Script-Based)

On Windows, symlinks require administrator privileges, so `.cmd` wrappers are used instead:

```
~/.vite-plus/bin/
├── vp.cmd     → "@echo off\n...vp.exe %*"
├── vp         → "#!/bin/sh\n...exec vp.exe \"$@\""   (Git Bash)
├── node.cmd   → "@echo off\n...vp.exe env exec node -- %*"
├── node       → "#!/bin/sh\n...exec vp.exe env exec node -- \"$@\""
└── ...
```

Each tool gets two files:

1. `.cmd` wrapper for `cmd.exe` and PowerShell
2. Shell script (no extension) for Git Bash

The `.cmd` wrappers set `VITE_PLUS_SHIM_WRAPPER=1` to signal that argument normalization is needed (the `--` separator must be stripped).

**Relevant code**: `crates/vite_global_cli/src/commands/env/setup.rs:create_windows_shim()` (line 245)

## Proposed Solution

### Overview

Create a minimal Windows trampoline executable (`~50-150KB`) that replaces both `.cmd` and shell script wrappers. The trampoline:

1. Reads its own filename to determine the tool name (e.g., `node.exe` → `"node"`)
2. Locates `vp.exe` at a known relative path (`../current/bin/vp.exe`)
3. Sets `VITE_PLUS_SHIM_TOOL` environment variable so `vp.exe` enters shim mode
4. Spawns `vp.exe` as a child process, passing through all arguments
5. Installs a Ctrl+C handler that ignores signals (the child handles them)
6. Waits for the child to exit and propagates its exit code

### Crate Structure

```
crates/vite_trampoline/
├── Cargo.toml
├── src/
│   └── main.rs        # Single-file trampoline binary
```

### Trampoline Binary Design

```rust
//! Minimal Windows trampoline for vite-plus shims.
//!
//! This binary is copied and renamed for each shim tool (node.exe, npm.exe, etc.).
//! It detects the tool name from its own filename, then spawns vp.exe with the
//! VITE_PLUS_SHIM_TOOL environment variable set.

use std::env;
use std::path::PathBuf;
use std::process::{self, Command};

#[cfg(windows)]
use windows::Win32::System::Console::{SetConsoleCtrlHandler, PHANDLER_ROUTINE};
#[cfg(windows)]
use windows::Win32::Foundation::{BOOL, TRUE};

fn main() {
    // 1. Determine tool name from our own executable filename
    let exe_path = env::current_exe().unwrap_or_else(|_| process::exit(1));
    let tool_name = exe_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| process::exit(1));

    // 2. Locate vp.exe: <bin_dir>/../current/bin/vp.exe
    let bin_dir = exe_path.parent().unwrap_or_else(|| process::exit(1));
    let vp_home = bin_dir.parent().unwrap_or_else(|| process::exit(1));
    let vp_exe = vp_home.join("current").join("bin").join("vp.exe");

    if !vp_exe.exists() {
        eprintln!("vite-plus: vp.exe not found at {}", vp_exe.display());
        process::exit(1);
    }

    // 3. Install Ctrl+C handler that ignores signals (child will handle them)
    #[cfg(windows)]
    install_ctrl_handler();

    // 4. Spawn vp.exe with VITE_PLUS_SHIM_TOOL set
    let status = Command::new(&vp_exe)
        .env("VITE_PLUS_SHIM_TOOL", tool_name)
        .args(env::args_os().skip(1))
        .status();

    // 5. Propagate exit code
    match status {
        Ok(s) => process::exit(s.code().unwrap_or(1)),
        Err(e) => {
            eprintln!("vite-plus: failed to execute vp.exe: {}", e);
            process::exit(1);
        }
    }
}

#[cfg(windows)]
fn install_ctrl_handler() {
    unsafe extern "system" fn handler(_ctrl_type: u32) -> BOOL {
        TRUE // Signal handled (ignored) - child process will receive it
    }
    unsafe {
        let _ = SetConsoleCtrlHandler(Some(handler), true);
    }
}
```

### Size Optimization

The `Cargo.toml` profile settings minimize binary size:

```toml
[package]
name = "vite_trampoline"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "vp-shim"
path = "src/main.rs"

[dependencies]
# Only needed for SetConsoleCtrlHandler
[target.'cfg(windows)'.dependencies]
windows = { version = "0.61", features = ["Win32_System_Console", "Win32_Foundation"] }

[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Better optimization
panic = "abort"        # No unwinding overhead
strip = true           # Remove symbols
```

**Expected binary size**: ~50-150KB per copy (vs. ~200 bytes for `.cmd` files, but with dramatically better UX).

**Note on further optimization**: The uv-trampoline achieves ~40KB by using `panic="immediate-abort"` (nightly-only), `#![no_main]`, raw Win32 APIs, and `ufmt` instead of `core::fmt`. These techniques are available if smaller binaries are desired, but the simpler stable-Rust approach (~100-150KB) is recommended initially for maintainability. The binary can be optimized further in a follow-up if needed.

### How Ctrl+C Handling Works

The trampoline installs a console control handler that returns `TRUE` (meaning "I handled the event"). This prevents Windows from terminating the trampoline process. The key insight:

1. When Ctrl+C is pressed, Windows sends the `CTRL_C_EVENT` to **all processes** in the console group
2. The trampoline's handler returns `TRUE` → trampoline stays alive
3. The child process (`vp.exe` → Node.js) receives the **same** `CTRL_C_EVENT`
4. The child decides how to handle it (typically exits gracefully)
5. The trampoline detects the child's exit and propagates its exit code

**No "Terminate batch job?" prompt** because there is no batch file involved.

### Integration with Existing Shim Detection

The trampoline uses the existing `VITE_PLUS_SHIM_TOOL` environment variable, which is already supported by `detect_shim_tool()` in `crates/vite_global_cli/src/shim/mod.rs` (line 100-132):

```
Trampoline (node.exe)
  → sets VITE_PLUS_SHIM_TOOL=node
  → spawns vp.exe with args
    → detect_shim_tool() reads env var → "node"
    → dispatch("node", args)
    → resolves Node.js version, executes real node
```

This means **no changes to the shim dispatch logic** are needed. The trampoline is a drop-in replacement for `.cmd` wrappers.

### Special Case: `vp.exe` Itself

The main `vp` command also currently uses a `.cmd` wrapper on Windows. For this case, the trampoline does NOT set `VITE_PLUS_SHIM_TOOL` (since `vp` is not a shim tool). Instead, it simply forwards all arguments:

```
vp.exe (trampoline in bin/)
  → spawns ../current/bin/vp.exe with args (no VITE_PLUS_SHIM_TOOL)
  → vp.exe runs in normal CLI mode
```

The trampoline can detect this by checking if its own filename is `vp.exe` and skipping the env var.

## Changes Required

### 1. New Crate: `crates/vite_trampoline/`

- Minimal Rust binary as described above
- Windows-only (conditionally compiled or only built for Windows targets)
- Added to workspace `Cargo.toml`

### 2. Shim Creation: `crates/vite_global_cli/src/commands/env/setup.rs`

Replace `create_windows_shim()` (line 245-284):

**Before**: Write `.cmd` + shell script per tool
**After**: Copy trampoline binary as `<tool>.exe`

```rust
#[cfg(windows)]
async fn create_windows_shim(
    _source: &std::path::Path,
    bin_dir: &vite_path::AbsolutePath,
    tool: &str,
) -> Result<(), Error> {
    let trampoline_src = get_trampoline_path()?;  // bundled vp-shim.exe
    let shim_path = bin_dir.join(format!("{tool}.exe"));
    tokio::fs::copy(&trampoline_src, &shim_path).await?;
    Ok(())
}
```

Also update the `vp.cmd` creation block (line 130-162) to use a trampoline copy instead.

### 3. Package Binary Shims: `crates/vite_global_cli/src/commands/env/global_install.rs`

Update `create_package_shim()` Windows branch (line 407-443) to copy trampoline as `<package>.exe` instead of writing `.cmd` + shell scripts.

### 4. Install Scripts

**`packages/cli/install.ps1`**:

- Include `vp-shim.exe` in the platform npm package
- Copy it as `vp.exe` to `~/.vite-plus/bin/` instead of writing `vp.cmd`
- Remove `.cmd` wrapper creation logic

**`packages/cli/install.sh`** (Windows/MSYS section):

- Similar changes for Git Bash on Windows

### 5. CI/Release Pipeline

**`.github/workflows/release.yml`** and **`.github/actions/build-upstream/action.yml`**:

- Add build step for `vite_trampoline` crate targeting Windows (x86_64, aarch64)
- Include `vp-shim.exe` in platform-specific npm packages alongside `vp.exe`

### 6. Trampoline Distribution

The trampoline binary must be bundled with the vite-plus distribution:

**Option A (Recommended)**: Include pre-built `vp-shim.exe` in the platform npm packages:

```
@voidzero-dev/vite-plus-win32-x64-msvc/
├── vp.exe          # Main CLI binary
└── vp-shim.exe     # Trampoline template
```

During `vp env setup`, `vp-shim.exe` is copied and renamed for each tool.

**Option B**: Embed the trampoline binary as a byte array in the `vp` binary itself:

```rust
const TRAMPOLINE_BYTES: &[u8] = include_bytes!("../../../target/release/vp-shim.exe");
```

Option A is simpler and avoids bloating the `vp` binary.

## Alternatives Considered

### 1. NTFS Hardlinks (Rejected)

Hardlink `node.exe` → `vp.exe` so they share the same file data. Since `vp.exe` already does `argv[0]` detection, this would work without any new binary.

**Why rejected**:

- Hardlinks resolve to physical file inodes, not through directory junctions
- When `~/.vite-plus/current` junction is re-pointed during upgrade, hardlinks in `bin/` still reference the **old** binary in the old version directory
- Every `vp` upgrade would require recreating all hardlinks
- On Unix, symlinks follow `current/` dynamically; hardlinks cannot

### 2. Windows Symbolic Links (Rejected)

Use symlinks like Unix: `node.exe` → `../current/bin/vp.exe`.

**Why rejected**:

- Requires administrator privileges or Developer Mode on Windows
- Not reliable for all users
- Cannot be assumed in a general installer

### 3. PowerShell `.ps1` Scripts (Rejected)

Replace `.cmd` with `.ps1` scripts that don't have the Ctrl+C issue.

**Why rejected**:

- `where.exe` and `which` do not find `.ps1` files as executables
- Only works in PowerShell, not `cmd.exe` or other shells
- Confirmed in issue #835 comments

### 4. Copy `vp.exe` as Each Shim (Rejected)

Copy the full `vp.exe` binary (~5-10MB) as `node.exe`, `npm.exe`, etc.

**Why rejected**:

- Significant disk space waste (~50MB for 5-10 tools)
- Must re-copy all tools on every `vp` upgrade
- Trampoline achieves the same result at ~100KB per tool

### 5. Keep `.cmd` Wrappers (Status Quo, Rejected)

Accept the `Terminate batch job (Y/N)?` prompt.

**Why rejected**:

- Poor user experience, especially for development servers (Ctrl+C is frequent)
- The prompt can appear multiple times in chained `.cmd` invocations
- Other modern CLI tools (uv, Volta) have moved away from `.cmd` wrappers

## Migration Strategy

### Upgrade Path

When users upgrade vite-plus on Windows:

1. `vp env setup --refresh` detects and removes old `.cmd` and shell script wrappers
2. Creates new `.exe` trampoline copies for all registered shims
3. Logs migration activity for debugging

### Cleanup Logic

```rust
// In setup --refresh on Windows:
// 1. Remove old .cmd wrappers
for tool in ["vp", "node", "npm", "npx", "vpx"] {
    let cmd_path = bin_dir.join(format!("{tool}.cmd"));
    if cmd_path.exists() {
        fs::remove_file(&cmd_path).await?;
    }
}
// 2. Remove old shell wrappers (extensionless files that are scripts, not exe)
// Check if file is a script by reading first bytes
// 3. Create new .exe trampolines
```

### Backward Compatibility

- Old `.cmd` wrappers continue to work until explicitly refreshed
- The `VITE_PLUS_SHIM_TOOL` env var interface is unchanged
- No changes to Unix behavior (symlinks unaffected)

## Comparison with uv-trampoline

| Aspect              | uv-trampoline                                | vite-plus trampoline                 |
| ------------------- | -------------------------------------------- | ------------------------------------ |
| **Purpose**         | Launch Python with embedded script           | Forward to `vp.exe`                  |
| **Complexity**      | High (PE resources, zipimport)               | Low (argv[0] + spawn)                |
| **Data embedding**  | PE resources (kind, path, script ZIP)        | None (uses filename + relative path) |
| **Toolchain**       | Nightly Rust (for `panic="immediate-abort"`) | Stable Rust                          |
| **Binary size**     | 39-47 KB                                     | ~100-150 KB (estimated)              |
| **Entry point**     | `#![no_main]` + `mainCRTStartup`             | Standard `fn main()`                 |
| **Formatting**      | `ufmt` (no `core::fmt`)                      | Standard `eprintln!`                 |
| **Ctrl+C handling** | `SetConsoleCtrlHandler` → ignore             | Same approach                        |
| **Exit code**       | `GetExitCodeProcess` → `exit()`              | `Command::status()` → `exit()`       |

The vite-plus trampoline is significantly simpler because it doesn't need to embed data in PE resources. It just needs to:

1. Know its own name (from the filesystem)
2. Find `vp.exe` (fixed relative path)
3. Spawn and wait

This makes it maintainable with stable Rust and standard library APIs.

## Future Optimizations

If the ~100-150KB binary size becomes a concern (e.g., many globally installed packages), the following optimizations can be applied incrementally:

1. **Switch to nightly Rust** with `panic="immediate-abort"` and `#![no_main]` (~50KB savings)
2. **Use `ufmt`** instead of `core::fmt` for error messages (~50KB savings)
3. **Use raw Win32 `CreateProcessW`** instead of `std::process::Command` (further savings)
4. **Pre-build and check in** trampoline binaries (like uv does) to simplify CI

These would bring the binary to ~40-50KB, matching uv-trampoline, but are not necessary for the initial implementation.

## Testing

### Unit Tests

- Test that the trampoline binary can be built for Windows targets
- Test tool name extraction from various filenames
- Test `vp.exe` path resolution from trampoline location

### Integration Tests (Windows CI)

1. Build and install vite-plus with trampoline shims
2. Verify `node --version` works through the trampoline
3. Verify `npm --version` works through the trampoline
4. Verify Ctrl+C does NOT produce "Terminate batch job?" prompt
5. Verify exit codes are properly propagated
6. Verify globally installed package binaries work through trampolines

### Migration Tests

1. Start with `.cmd` wrappers installed
2. Run `vp env setup --refresh`
3. Verify `.cmd` files are removed and `.exe` trampolines are created
4. Verify all shims work after migration

## References

- [Issue #835](https://github.com/voidzero-dev/vite-plus/issues/835): Original feature request with video reproduction
- [uv-trampoline](https://github.com/astral-sh/uv/tree/main/crates/uv-trampoline): Reference implementation by astral-sh
- [Volta shims](https://github.com/volta-cli/volta): Similar Node.js version manager with shim binaries
- [RFC: env-command](./env-command.md): Existing shim architecture documentation
- [RFC: global-cli-rust-binary](./global-cli-rust-binary.md): `vp` binary architecture
