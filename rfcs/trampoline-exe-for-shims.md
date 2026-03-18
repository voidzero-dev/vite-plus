# RFC: Windows Trampoline `.exe` for Shims

## Status

Implemented

## Summary

Replace Windows `.cmd` wrapper scripts with lightweight trampoline `.exe` binaries for all shim tools (`vp`, `node`, `npm`, `npx`, `vpx`, and globally installed package binaries). This eliminates the `Terminate batch job (Y/N)?` prompt that appears when users press Ctrl+C, providing the same clean signal behavior as direct `.exe` invocation.

## Motivation

### The Problem

On Windows, the vite-plus CLI previously exposed tools through `.cmd` batch file wrappers:

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

## Architecture

### Unix (Symlink-Based — Unchanged)

On Unix, shims are symlinks to the `vp` binary. The binary detects the tool name from `argv[0]`:

```
~/.vite-plus/bin/
├── vp   → ../current/bin/vp     (symlink)
├── node → ../current/bin/vp     (symlink)
├── npm  → ../current/bin/vp     (symlink)
└── npx  → ../current/bin/vp     (symlink)
```

### Windows (Trampoline `.exe` Files)

```
~/.vite-plus/bin/
├── vp.exe       # Trampoline → spawns current\bin\vp.exe
├── node.exe     # Trampoline → sets VITE_PLUS_SHIM_TOOL=node, spawns vp.exe
├── npm.exe      # Trampoline → sets VITE_PLUS_SHIM_TOOL=npm, spawns vp.exe
├── npx.exe      # Trampoline → sets VITE_PLUS_SHIM_TOOL=npx, spawns vp.exe
├── vpx.exe      # Trampoline → sets VITE_PLUS_SHIM_TOOL=vpx, spawns vp.exe
└── tsc.exe      # Trampoline → sets VITE_PLUS_SHIM_TOOL=tsc, spawns vp.exe (package shim)
```

Each trampoline is a copy of `vp-shim.exe` (the template binary distributed alongside `vp.exe`).

**Note**: npm-installed packages (via `npm install -g`) still use `.cmd` wrappers because they lack `PackageMetadata` and need to point directly at npm's generated scripts.

## Implementation

### Crate Structure

```
crates/vite_trampoline/
├── Cargo.toml      # Zero external dependencies
├── src/
│   └── main.rs     # ~90 lines, single-file binary
```

### Trampoline Binary

The trampoline has **zero external dependencies** — the Win32 FFI call (`SetConsoleCtrlHandler`) is declared inline to avoid the heavy `windows`/`windows-core` crates. It also avoids `core::fmt` (~100KB overhead) by never using `format!`, `eprintln!`, `println!`, or `.unwrap()`.

```rust
use std::{env, process::{self, Command}};

fn main() {
    // 1. Determine tool name from own filename (e.g., node.exe → "node")
    let exe_path = env::current_exe().unwrap_or_else(|_| process::exit(1));
    let tool_name = exe_path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or_else(|| process::exit(1));

    // 2. Locate vp.exe at ../current/bin/vp.exe
    let bin_dir = exe_path.parent().unwrap_or_else(|| process::exit(1));
    let vp_home = bin_dir.parent().unwrap_or_else(|| process::exit(1));
    let vp_exe = vp_home.join("current").join("bin").join("vp.exe");

    // 3. Install Ctrl+C handler (ignores signal; child handles it)
    install_ctrl_handler();

    // 4. Spawn vp.exe with env vars
    let mut cmd = Command::new(&vp_exe);
    cmd.args(env::args_os().skip(1));
    cmd.env("VITE_PLUS_HOME", vp_home);

    if tool_name != "vp" {
        cmd.env("VITE_PLUS_SHIM_TOOL", tool_name);
        cmd.env_remove("VITE_PLUS_TOOL_RECURSION");
    }

    // 5. Propagate exit code (error message via write_all, not eprintln!)
    match cmd.status() {
        Ok(s) => process::exit(s.code().unwrap_or(1)),
        Err(_) => {
            use std::io::Write;
            let mut stderr = std::io::stderr().lock();
            let _ = stderr.write_all(b"vite-plus: failed to execute ");
            let _ = stderr.write_all(vp_exe.as_os_str().as_encoded_bytes());
            let _ = stderr.write_all(b"\n");
            process::exit(1);
        }
    }
}

fn install_ctrl_handler() {
    type HandlerRoutine = unsafe extern "system" fn(ctrl_type: u32) -> i32;
    unsafe extern "system" {
        fn SetConsoleCtrlHandler(handler: Option<HandlerRoutine>, add: i32) -> i32;
    }
    unsafe extern "system" fn handler(_ctrl_type: u32) -> i32 { 1 }
    unsafe { SetConsoleCtrlHandler(Some(handler), 1); }
}
```

### Size Optimization

| Technique                                                                             | Savings                    | Status |
| ------------------------------------------------------------------------------------- | -------------------------- | ------ |
| Zero external dependencies (raw FFI)                                                  | ~20KB (vs `windows` crate) | Done   |
| No direct `core::fmt` usage (avoid `eprintln!`/`format!`/`.unwrap()`)                 | Marginal                   | Done   |
| Workspace profile: `lto="fat"`, `codegen-units=1`, `strip="symbols"`, `panic="abort"` | Inherited                  | Done   |
| Per-package `opt-level="z"` (optimize for size)                                       | ~5-10%                     | Done   |

**Binary size**: ~200KB on Windows. The floor is set by `std::process::Command` which internally pulls in `core::fmt` for error formatting regardless of whether our code uses it. Further reduction to ~40-50KB (matching uv-trampoline) would require replacing `Command` with raw `CreateProcessW` and using nightly Rust (see Future Optimizations).

### Environment Variables

The trampoline sets three env vars before spawning `vp.exe`:

| Variable                   | When                       | Purpose                                                                        |
| -------------------------- | -------------------------- | ------------------------------------------------------------------------------ |
| `VITE_PLUS_HOME`           | Always                     | Tells vp.exe the install directory (derived from `bin_dir.parent()`)           |
| `VITE_PLUS_SHIM_TOOL`      | Tool shims only (not "vp") | Tells vp.exe to enter shim dispatch mode for the named tool                    |
| `VITE_PLUS_TOOL_RECURSION` | Removed for tool shims     | Clears the recursion marker for fresh version resolution in nested invocations |

### Ctrl+C Handling

The trampoline installs a console control handler that returns `TRUE` (1):

1. When Ctrl+C is pressed, Windows sends `CTRL_C_EVENT` to **all processes** in the console group
2. The trampoline's handler returns 1 (TRUE) → trampoline stays alive
3. The child process (`vp.exe` → Node.js) receives the **same** event
4. The child decides how to handle it (typically exits gracefully)
5. The trampoline detects the child's exit and propagates its exit code

**No "Terminate batch job?" prompt** because there is no batch file involved.

### Integration with Shim Detection

`detect_shim_tool()` in `shim/mod.rs` checks `VITE_PLUS_SHIM_TOOL` env var **before** `argv[0]`:

```
Trampoline (node.exe)
  → sets VITE_PLUS_SHIM_TOOL=node, VITE_PLUS_HOME=..., removes VITE_PLUS_TOOL_RECURSION
  → spawns current/bin/vp.exe with original args
    → detect_shim_tool() reads env var → "node"
    → dispatch("node", args)
    → resolves Node.js version, executes real node
```

### Running Exe Overwrite

When `vp env setup --refresh` is invoked through the trampoline (`~/.vite-plus/bin/vp.exe`), the trampoline is still running. Windows prevents overwriting a running `.exe`. The solution:

1. Rename existing `vp.exe` to `vp.exe.<unix_timestamp>.old`
2. Copy new trampoline to `vp.exe`
3. Best-effort cleanup of all `*.old` files in the bin directory

### Distribution

The trampoline binary (`vp-shim.exe`) is distributed alongside `vp.exe`:

```
~/.vite-plus/current/bin/
├── vp.exe          # Main CLI binary
└── vp-shim.exe     # Trampoline template (copied as shims)
```

Included in:

- Platform npm packages (`@voidzero-dev/vite-plus-cli-win32-x64-msvc`)
- Release artifacts (`.github/workflows/release.yml`)
- `install.ps1` and `install.sh` (both local dev and download paths)
- `extract_platform_package()` in the upgrade path

### Legacy Fallback

When installing a pre-trampoline version (no `vp-shim.exe` in the package):

- `install.ps1` falls back to creating `.cmd` + shell script wrappers
- Stale trampoline `.exe` shims from a newer install are removed (`.exe` takes precedence over `.cmd` on Windows PATH)

## Comparison with uv-trampoline

| Aspect              | uv-trampoline                            | vite-plus trampoline                 |
| ------------------- | ---------------------------------------- | ------------------------------------ |
| **Purpose**         | Launch Python with embedded script       | Forward to `vp.exe`                  |
| **Complexity**      | High (PE resources, zipimport)           | Low (filename + spawn)               |
| **Data embedding**  | PE resources (kind, path, script ZIP)    | None (uses filename + relative path) |
| **Dependencies**    | `windows` crate (unsafe, no CRT)         | Zero (raw FFI declaration)           |
| **Toolchain**       | Nightly Rust (`panic="immediate-abort"`) | Stable Rust                          |
| **Binary size**     | 39-47 KB                                 | ~200 KB                              |
| **Entry point**     | `#![no_main]` + `mainCRTStartup`         | Standard `fn main()`                 |
| **Error output**    | `ufmt` (no `core::fmt`)                  | `write_all` (no `core::fmt`)         |
| **Ctrl+C handling** | `SetConsoleCtrlHandler` → ignore         | Same approach                        |
| **Exit code**       | `GetExitCodeProcess` → `exit()`          | `Command::status()` → `exit()`       |

The vite-plus trampoline is significantly simpler because it doesn't need to embed data in PE resources — it just reads its own filename, finds `vp.exe` at a fixed relative path, and spawns it. The ~150KB size difference from uv-trampoline comes from `std::process::Command` (which internally pulls in `core::fmt`) versus raw `CreateProcessW` with nightly-only `#![no_main]`.

## Alternatives Considered

### 1. NTFS Hardlinks (Rejected)

Hardlinks resolve to physical file inodes, not through directory junctions. After `vp` upgrade re-points `current`, hardlinks in `bin/` still reference the old binary.

### 2. Windows Symbolic Links (Rejected)

Requires administrator privileges or Developer Mode. Not reliable for all users.

### 3. PowerShell `.ps1` Scripts (Rejected)

`where.exe` and `which` do not find `.ps1` files. Only works in PowerShell.

### 4. Copy `vp.exe` as Each Shim (Rejected)

~5-10MB per copy. Trampoline achieves the same result at ~100KB.

### 5. `windows` Crate for FFI (Rejected)

Adds ~100KB to the binary for a single `SetConsoleCtrlHandler` call. Raw FFI declaration is sufficient.

## Future Optimizations

If the ~100KB binary size needs to be reduced further:

1. **Switch to nightly Rust** with `panic="immediate-abort"` and `#![no_main]` + `mainCRTStartup` (~50KB savings)
2. **Use raw Win32 `CreateProcessW`** instead of `std::process::Command` (eliminates most of std's process machinery)
3. **Pre-build and check in** trampoline binaries (like uv does) to decouple the trampoline build from the workspace toolchain

These would bring the binary to ~40-50KB, matching uv-trampoline, at the cost of requiring a nightly toolchain and more unsafe code.

## References

- [Issue #835](https://github.com/voidzero-dev/vite-plus/issues/835): Original feature request with video reproduction
- [uv-trampoline](https://github.com/astral-sh/uv/tree/main/crates/uv-trampoline): Reference implementation by astral-sh (~40KB with nightly Rust)
- [RFC: env-command](./env-command.md): Shim architecture documentation
- [RFC: upgrade-command](./upgrade-command.md): Upgrade/rollback flow
