# RFC: Windows Trampoline `.exe` for Shims

## Status

Implemented

## Summary

Replace Windows `.cmd` wrapper scripts with lightweight trampoline `.exe` binaries for all shim tools (`vp`, `node`, `npm`, `npx`, `corepack`, `vpx`, `vpr`, and globally installed package binaries). This eliminates the `Terminate batch job (Y/N)?` prompt that appears when users press Ctrl+C, providing the same clean signal behavior as direct `.exe` invocation.

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
2. Running `<DATA>/current/bin/vp.exe dev` directly does **NOT** show the prompt
3. Running `npm.cmd run dev` shows the prompt; running `npm.ps1 run dev` does not
4. The prompt can appear multiple times when `.cmd` wrappers chain (e.g., `vp.cmd` → `npm.cmd`)

### Why `.ps1` Scripts Are Not Sufficient

PowerShell `.ps1` scripts avoid the Ctrl+C issue but have critical limitations:

- `where.exe` and `which` do not discover `.ps1` files as executables
- Only work in PowerShell, not in `cmd.exe`, Git Bash, or other shells
- Cannot serve as universal shims

## Architecture

This RFC uses the `<BIN>`, `<DATA>`, and `<CACHE>` roots from the
[directory layout RFC](./directory-layout.md).

### Unix (Symlink-Based — Unchanged)

On Unix, shims are symlinks to the `vp` binary. The binary detects the tool name from `argv[0]`:

```
<BIN>/
├── vp       → <DATA>/current/bin/vp     (symlink)
├── node     → <DATA>/current/bin/vp     (symlink)
├── npm      → <DATA>/current/bin/vp     (symlink)
├── npx      → <DATA>/current/bin/vp     (symlink)
├── corepack → <DATA>/current/bin/vp     (symlink)
├── vpx      → <DATA>/current/bin/vp     (symlink)
└── vpr      → <DATA>/current/bin/vp     (symlink)
```

### Windows (Trampoline `.exe` Files)

```
<BIN>/
├── vp.exe       # Trampoline executable
├── vp.shim      # Directory-layout sidecar for vp.exe
├── node.exe     # Trampoline executable
├── node.shim    # Directory-layout sidecar for node.exe
├── npm.exe      # Trampoline executable
├── npm.shim     # Directory-layout sidecar for npm.exe
└── ...

<DATA>/current/bin/
├── vp.exe       # Main CLI binary
└── vp-shim.exe  # Trampoline template
```

Each trampoline is a copy of `vp-shim.exe`. Each copy has a sidecar with the
same file stem. For example, `node.exe` reads `node.shim`. The tool name is not
stored in the sidecar.

A split-layout sidecar has this format:

```text
vite-plus-shim-v1
layout=split
data=C:\Users\alice\AppData\Local\vite-plus\data
cache=C:\Users\alice\AppData\Local\vite-plus\cache
```

A `VP_HOME=C:\Tools\vite-plus` install uses the single-root layout:

```text
vite-plus-shim-v1
layout=single-root
data=C:\Tools\vite-plus
cache=C:\Tools\vite-plus\cache
```

The exact `vite-plus-shim-v1` header is required. The trampoline and ownership
checks reject unversioned sidecars. The sidecar is the source of truth for the
layout and also records that Vite+ owns the adjacent executable.

**Note**: npm-installed packages (via `npm install -g`) still use `.cmd` wrappers because they lack `PackageMetadata` and need to point directly at npm's generated scripts.

## Implementation

### Crate Structure

```
crates/vp_trampoline/
├── Cargo.toml      # Zero external dependencies
├── src/
│   └── main.rs     # Sidecar parser, launcher, and portable tests
```

### Trampoline Binary

The trampoline has **zero external dependencies**. It declares the Win32
`SetConsoleCtrlHandler` call inline to avoid the `windows` and `windows-core`
crates. It also avoids direct `core::fmt` use. It does not use `format!`,
`eprintln!`, `println!`, or `.unwrap()` in the production path.

The trampoline performs these steps:

1. Read its executable path and get the tool name from the file stem.
2. Read the same-stem `.shim` file. Require the `vite-plus-shim-v1` header and
   parse the layout, data root, and cache root.
3. Resolve the child executable as `<DATA>/current/bin/vp.exe`.
4. Pin the recorded layout in the child environment. A `single-root` sidecar
   sets `VP_HOME`. A `split` sidecar removes `VP_HOME` and sets `VP_DATA_DIR`,
   `VP_BIN_DIR`, and `VP_CACHE_DIR`.
5. Install the Ctrl+C handler, start the child, and propagate its exit code.

The trampoline fails if the sidecar is missing, malformed, unversioned, or
points to a missing payload. It does not infer the layout from directory paths.

### Size Optimization

| Technique                                                                             | Savings                    | Status |
| ------------------------------------------------------------------------------------- | -------------------------- | ------ |
| Zero external dependencies (raw FFI)                                                  | ~20KB (vs `windows` crate) | Done   |
| No direct `core::fmt` usage (avoid `eprintln!`/`format!`/`.unwrap()`)                 | Marginal                   | Done   |
| Workspace profile: `lto="fat"`, `codegen-units=1`, `strip="symbols"`, `panic="abort"` | Inherited                  | Done   |
| Per-package `opt-level="z"` (optimize for size)                                       | ~5-10%                     | Done   |

**Binary size**: ~200KB on Windows. The floor is set by `std::process::Command` which internally pulls in `core::fmt` for error formatting regardless of whether our code uses it. Further reduction to ~40-50KB (matching uv-trampoline) would require replacing `Command` with raw `CreateProcessW` and using nightly Rust (see Future Optimizations).

### Environment Variables

The trampoline pins the selected directory layout before it starts `vp.exe`:

| Variable            | When                       | Purpose                                                                        |
| ------------------- | -------------------------- | ------------------------------------------------------------------------------ |
| `VP_HOME`           | `single-root` sidecar      | Pins the data root as the single install root                                  |
| `VP_HOME`           | `split` sidecar            | Removed so it cannot override the recorded split layout                        |
| `VP_DATA_DIR`       | `split` sidecar            | Pins the recorded data root                                                    |
| `VP_BIN_DIR`        | `split` sidecar            | Pins the trampoline executable directory                                       |
| `VP_CACHE_DIR`      | `split` sidecar            | Pins the recorded cache root                                                   |
| `VP_SHIM_TOOL`      | Tool shims only (not `vp`) | Tells `vp.exe` to enter shim dispatch mode for the named tool                  |
| `VP_TOOL_RECURSION` | Removed for tool shims     | Clears the recursion marker for fresh version resolution in nested invocations |

### Ctrl+C Handling

The trampoline installs a console control handler that returns `TRUE` (1):

1. When Ctrl+C is pressed, Windows sends `CTRL_C_EVENT` to **all processes** in the console group
2. The trampoline's handler returns 1 (TRUE) → trampoline stays alive
3. The child process (`vp.exe` → Node.js) receives the **same** event
4. The child decides how to handle it (typically exits gracefully)
5. The trampoline detects the child's exit and propagates its exit code

**No "Terminate batch job?" prompt** because there is no batch file involved.

### Integration with Shim Detection

`detect_shim_tool()` in `shim/mod.rs` checks `VP_SHIM_TOOL` env var **before** `argv[0]`:

```
Trampoline (<BIN>/node.exe + <BIN>/node.shim)
  → reads the recorded layout, data root, and cache root
  → pins the layout, sets VP_SHIM_TOOL=node, and removes VP_TOOL_RECURSION
  → spawns <DATA>/current/bin/vp.exe with the original args
    → detect_shim_tool() reads env var → "node"
    → dispatch("node", args)
    → resolves Node.js version, executes real node
```

### Running Exe Overwrite

When `vp env setup --refresh` is invoked through the trampoline
(`<BIN>/vp.exe`), the trampoline is still running. Windows prevents
overwriting a running `.exe`. The solution:

1. Rename existing `vp.exe` to `vp.exe.<unix_timestamp>.old`
2. Copy new trampoline to `vp.exe`
3. Best-effort cleanup of all `*.old` files in the bin directory

### Upgrade Refresh

During `vp upgrade`, after the `current` link is swapped to the new version, `vp env setup --refresh` is invoked to regenerate all trampoline `.exe` files. This ensures that when the trampoline binary (`vp-shim.exe`) changes between versions, all shims pick up the new version:

1. **Core shims** (`vp.exe`, `node.exe`, `npm.exe`, `npx.exe`, `corepack.exe`, `vpx.exe`, `vpr.exe`) are refreshed by the standard `--refresh` logic.
2. **Package shims** (e.g., `tsc.exe`, `eslint.exe`, installed via `vp install -g`) are discovered by scanning `<DATA>/bins/` for `BinConfig` entries with `source: Vp`, and each `.exe` is replaced with the new trampoline.

Package shims installed via npm interception (`source: Npm`) use `.cmd` wrappers, not trampoline `.exe` files, and are not affected by this refresh.

Additionally, re-installing a global package (`vp install -g <pkg>`) always re-copies the current trampoline, ensuring the shim stays up to date even without a full upgrade.

### Distribution

The trampoline binary (`vp-shim.exe`) is distributed alongside `vp.exe`:

```
<DATA>/current/bin/
├── vp.exe          # Main CLI binary
└── vp-shim.exe     # Trampoline template (copied as shims)
```

Included in:

- Platform npm packages (`@voidzero-dev/vite-plus-cli-win32-x64-msvc`)
- Release artifacts (`.github/workflows/release.yml`)
- `install.ps1` and `install.sh` (both local dev and download paths)
- `extract_platform_package()` in the upgrade path

### Pre-Trampoline Release Fallback

When installing a pre-trampoline version (no `vp-shim.exe` in the package):

- `install.ps1` falls back to creating `.cmd` + shell script wrappers
- Stale trampoline `.exe` shims from a newer install are removed (`.exe` takes precedence over `.cmd` on Windows PATH)

## Comparison with uv-trampoline

| Aspect              | uv-trampoline                            | vite-plus trampoline              |
| ------------------- | ---------------------------------------- | --------------------------------- |
| **Purpose**         | Launch Python with embedded script       | Forward to `vp.exe`               |
| **Complexity**      | High (PE resources, zipimport)           | Low (filename + spawn)            |
| **Data embedding**  | PE resources (kind, path, script ZIP)    | Adjacent directory-layout sidecar |
| **Dependencies**    | `windows` crate (unsafe, no CRT)         | Zero (raw FFI declaration)        |
| **Toolchain**       | Nightly Rust (`panic="immediate-abort"`) | Stable Rust                       |
| **Binary size**     | 39-47 KB                                 | ~200 KB                           |
| **Entry point**     | `#![no_main]` + `mainCRTStartup`         | Standard `fn main()`              |
| **Error output**    | `ufmt` (no `core::fmt`)                  | `write_all` (no `core::fmt`)      |
| **Ctrl+C handling** | `SetConsoleCtrlHandler` → ignore         | Same approach                     |
| **Exit code**       | `GetExitCodeProcess` → `exit()`          | `Command::status()` → `exit()`    |

The vite-plus trampoline does not embed data in PE resources. It reads its own
filename and adjacent sidecar. It then resolves `vp.exe` under the recorded data
root. The ~150KB size difference from uv-trampoline comes from
`std::process::Command` (which internally pulls in `core::fmt`) versus raw
`CreateProcessW` with nightly-only `#![no_main]`.

## Alternatives Considered

### 1. NTFS Hardlinks (Rejected)

Hardlinks resolve to physical file inodes, not through directory junctions. After `vp` upgrade re-points `current`, hardlinks in `bin/` still reference the old binary.

### 2. Windows Symbolic Links (Rejected)

Requires administrator privileges or Developer Mode. Not reliable for all users.

### 3. PowerShell `.ps1` Scripts (Rejected)

`where.exe` and `which` do not find `.ps1` files. Only works in PowerShell.

### 4. Copy `vp.exe` as Each Shim (Rejected)

~5-10MB per copy. The trampoline achieves the same result at ~200KB.

### 5. `windows` Crate for FFI (Rejected)

Adds ~100KB to the binary for a single `SetConsoleCtrlHandler` call. Raw FFI declaration is sufficient.

## Future Optimizations

If the ~200KB binary size needs to be reduced further:

1. **Switch to nightly Rust** with `panic="immediate-abort"` and `#![no_main]` + `mainCRTStartup` (~50KB savings)
2. **Use raw Win32 `CreateProcessW`** instead of `std::process::Command` (eliminates most of std's process machinery)
3. **Pre-build and check in** trampoline binaries (like uv does) to decouple the trampoline build from the workspace toolchain

These would bring the binary to ~40-50KB, matching uv-trampoline, at the cost of requiring a nightly toolchain and more unsafe code.

## References

- [Issue #835](https://github.com/voidzero-dev/vite-plus/issues/835): Original feature request with video reproduction
- [uv-trampoline](https://github.com/astral-sh/uv/tree/main/crates/uv-trampoline): Reference implementation by astral-sh (~40KB with nightly Rust)
- [RFC: env-command](./env-command.md): Shim architecture documentation
- [RFC: upgrade-command](./upgrade-command.md): Upgrade/rollback flow
