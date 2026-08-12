# RFC: Windows Trampoline `.exe` for Shims

## Status

Implemented

## Summary

Replace Windows `.cmd` wrapper scripts with lightweight trampoline `.exe` binaries for all shim tools (`vp`, `node`, `npm`, `npx`, `vpx`, `vpr`, and globally installed package binaries). This eliminates the `Terminate batch job (Y/N)?` prompt that appears when users press Ctrl+C, providing the same clean signal behavior as direct `.exe` invocation.

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
├── Cargo.toml           # Package settings and release profile
├── Cargo.lock           # Lockfile for this standalone crate
├── .cargo/
│   └── config.toml      # build-std and artifact directory settings
├── src/
│   ├── main.rs          # Entry points and portable implementation
│   ├── win.rs           # Raw Win32 code for the no_main entry point
│   └── cmdline.rs       # Portable parsers and tests
```

The root `Cargo.toml` excludes this crate from the workspace. The crate must
stay outside the workspace for two reasons:

- The release profile sets `panic = "immediate-abort"`. Cargo ignores `panic`
  in per-package profile overrides. Thus, the crate needs a separate profile.
- The crate-local `.cargo/config.toml` enables build-std. Cargo reads this file
  only when it runs from the crate directory.

From the repository root, run:

```bash
node packages/tools/src/build-trampoline.ts --release [--target <triple>]
```

The crate config stores artifacts in the repository `target/` directory. It
sets `target-dir = "../../target"`. CI and `install-global-cli` find
`vp-shim.exe` in the same directory as workspace binaries. The build uses the
pinned nightly toolchain and the `rust-src` component. The repository
`rust-toolchain.toml` supplies both items.

### Trampoline Binary

The trampoline has no external dependencies. It declares all Win32 calls as
raw `extern "system"` functions from KERNEL32. Thus, it does not use the
`windows` or `windows-core` crate. It also does not use `core::fmt`.
Diagnostics use `WriteFile` and a small decimal formatter.

On Windows, the binary uses `#![no_main]` and exports `mainCRTStartup`. Thus,
the CRT startup and the `std` runtime do not initialize. `src/win.rs` uses this
sequence:

1. `GetModuleFileNameW` returns the shim path and tool name. The code replaces
   the `.exe` extension with `.shim` to find the sidecar.
2. `CreateFileW` and `ReadFile` load the UTF-8 sidecar. `GetFullPathNameW`
   makes long paths absolute. The code then adds the `\\?\` drive prefix or
   the `\\?\UNC\` network prefix. The parser requires the versioned header.
   It accepts the `single-root` and `split` layouts.
3. `SetEnvironmentVariableW` sets the directory layout. A single-root pointer
   sets `VP_HOME`. A split pointer removes `VP_HOME`. It sets `VP_DATA_DIR`,
   `VP_BIN_DIR`, and `VP_CACHE_DIR`. Tool shims also set `VP_SHIM_TOOL`. They
   remove `VP_TOOL_RECURSION`.
4. The child command line starts with `"<DATA>\current\bin\vp.exe"`. The code
   appends the raw `GetCommandLineW` text after the program argument. It uses
   the MSVC `argv[0]` rule. Quotation marks start or stop quoted mode.
   Backslashes do not escape characters. This preserves the exact UTF-16
   argument text from the caller.
5. `SetConsoleCtrlHandler` installs a handler that ignores Ctrl+C and
   Ctrl+Break. The child process handles these events.
6. `CreateProcessW` starts the child with inherited handles and startup
   information. The payload and sidecar paths use the same extended-length
   normalization. If the parent redirects standard I/O, the code makes the
   standard handles inheritable. It does this before `CreateProcessW`, as
   uv-trampoline and distlib do.
7. `WaitForSingleObject` waits for the child. `GetExitCodeProcess` reads its
   exit code. `ExitProcess` returns that code without changes.

For a critical launch failure, the trampoline reports the failed operation and
the applicable path. It includes the Windows error code when Windows supplies
one. If `vp.exe` is missing, it tells the user to reinstall Vite+ or run
`vp env setup`.

The non-Windows implementation uses `std::process::Command`. Portable tests use
the same sidecar parser. Unix shims are symlinks and do not use this binary.
The parser rejects missing, malformed, and unversioned sidecars. It does not
infer a layout from directory paths.

### Size Optimization

| Technique                                                                | Status |
| ------------------------------------------------------------------------ | ------ |
| Zero external dependencies (raw FFI, no `windows` crate)                 | Done   |
| No `core::fmt` (diagnostics via `WriteFile` + manual decimal formatter)  | Done   |
| Own profile: `opt-level="z"`, `lto="fat"`, `codegen-units=1`, `strip`    | Done   |
| build-std: recompile `std` with this profile (`-Zbuild-std`)             | Done   |
| `panic = "immediate-abort"` (no panic formatting, unwinding, backtrace)  | Done   |
| `#![no_main]` + `mainCRTStartup` (no CRT startup, no `std` runtime init) | Done   |
| Raw `CreateProcessW` instead of `std::process::Command`                  | Done   |

**Binary size**: 14,336 B on x86_64-pc-windows-msvc and
aarch64-pc-windows-msvc. This size includes the sidecar parser and diagnostics.
The x86_64 `std::process::Command` implementation was 221,696 B. See Size
Measurements and Build Constraints for all measurements. The executable imports
only KERNEL32.

### Environment Variables

The sidecar controls the directory environment inherited by `vp.exe`:

| Variable            | When                    | Trampoline action                                      |
| ------------------- | ----------------------- | ------------------------------------------------------ |
| `VP_HOME`           | Single-root layout      | Sets all Vite+ directories from the sidecar data root  |
| `VP_HOME`           | Split layout            | Removes the value so it cannot override separate roots |
| `VP_DATA_DIR`       | Split layout            | Sets the payload and state root                        |
| `VP_BIN_DIR`        | Split layout            | Sets the directory that contains the shim              |
| `VP_CACHE_DIR`      | Split layout            | Sets the cache root                                    |
| `VP_SHIM_TOOL`      | Tool shims, except `vp` | Selects the named tool for shim dispatch               |
| `VP_TOOL_RECURSION` | Tool shims              | Removes the value so nested shims resolve versions     |

### Ctrl+C Handling

The trampoline installs a console control handler that returns `TRUE` (1):

1. When Ctrl+C is pressed, Windows sends `CTRL_C_EVENT` to **all processes** in the console group
2. The trampoline's handler returns 1 (TRUE) → trampoline stays alive
3. The child process (`vp.exe` → Node.js) receives the **same** event
4. The child decides how to handle it (typically exits gracefully)
5. The trampoline detects the child's exit and propagates its exit code

**No "Terminate batch job?" prompt** because there is no batch file involved.

### Integration with Shim Detection

`detect_shim_tool()` in `shim/mod.rs` checks `VP_SHIM_TOOL` before it checks
`argv[0]`:

```
Trampoline (node.exe + node.shim)
  → loads the recorded directory layout
  → sets VP_SHIM_TOOL=node and the directory variables
  → removes VP_TOOL_RECURSION
  → spawns <DATA>/current/bin/vp.exe with the original argument tail
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

1. **Core shims** (`vp.exe`, `node.exe`, `npm.exe`, `npx.exe`, `vpx.exe`, `vpr.exe`) are refreshed by the standard `--refresh` logic.
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

| Aspect              | uv-trampoline                            | vite-plus trampoline                 |
| ------------------- | ---------------------------------------- | ------------------------------------ |
| **Purpose**         | Launch Python with embedded script       | Forward to `vp.exe`                  |
| **Complexity**      | High (PE resources, zipimport)           | Low (filename + spawn)               |
| **Data embedding**  | PE resources (kind, path, script ZIP)    | Adjacent directory-layout sidecar    |
| **Dependencies**    | `windows` crate (unsafe, no CRT)         | None (raw FFI declarations)          |
| **Toolchain**       | Nightly Rust (`panic="immediate-abort"`) | Nightly Rust (same technique)        |
| **Binary size**     | 39-47 KiB                                | 14 KiB                               |
| **Entry point**     | `#![no_main]` + `mainCRTStartup`         | `#![no_main]` + `mainCRTStartup`     |
| **Error output**    | `ufmt` (no `core::fmt`)                  | `WriteFile` + Win32 error codes      |
| **Ctrl+C handling** | `SetConsoleCtrlHandler` → ignore         | `SetConsoleCtrlHandler` → ignore     |
| **Exit code**       | `GetExitCodeProcess` → `exit()`          | `GetExitCodeProcess` → `ExitProcess` |

The Vite+ trampoline is smaller because it does not embed PE resources. It
normalizes only long sidecar and payload paths. It does not need job objects or
GUI subsystem support. It reads a small sidecar next to its file. It finds
`vp.exe` under the recorded data root and starts it. Both projects use the same
build method and entry-point structure.

## Alternatives Considered

### 1. NTFS Hardlinks (Rejected)

Hardlinks resolve to physical file inodes, not through directory junctions. After `vp` upgrade re-points `current`, hardlinks in `bin/` still reference the old binary.

### 2. Windows Symbolic Links (Rejected)

Requires administrator privileges or Developer Mode. Not reliable for all users.

### 3. PowerShell `.ps1` Scripts (Rejected)

`where.exe` and `which` do not find `.ps1` files. Only works in PowerShell.

### 4. Copy `vp.exe` as Each Shim (Rejected)

~5-10MB per copy. The trampoline achieves the same result in 14 KiB.

### 5. `windows` Crate for FFI (Rejected)

Adds ~100KB to the binary for a single `SetConsoleCtrlHandler` call. Raw FFI declaration is sufficient.

## Size Measurements and Build Constraints

We built each variant below with cargo-xwin. We measured each variant on
x86_64-pc-windows-msvc. The first two rows use the sidecar-aware `std`
implementation. The next five rows show earlier fixed-layout experiments. The
last row shows the current sidecar-aware raw implementation.

| Variant                                                                      | Toolchain | Size      |
| ---------------------------------------------------------------------------- | --------- | --------- |
| Sidecar-aware `std::process::Command`, precompiled `std`                     | stable    | 221,696 B |
| Same source + build-std + `panic="immediate-abort"`                          | nightly   | 82,432 B  |
| Fixed-layout `std` source + `#![no_main]` + `mainCRTStartup` + `atexit` stub | nightly   | 69,632 B  |
| Raw Win32 rewrite, normal `main`, stable, no build-std                       | stable    | 105,984 B |
| Raw Win32 rewrite, normal `main` + build-std                                 | nightly   | 13,824 B  |
| Raw Win32 rewrite + `#![no_main]`, no diagnostics                            | nightly   | 6,656 B   |
| Fixed-layout raw Win32 + `#![no_main]` + full diagnostics                    | nightly   | 8,192 B   |
| Sidecar-aware raw Win32 + `#![no_main]` + full diagnostics (shipped)         | nightly   | 14,336 B  |

For comparison, the uv-trampoline x64 console binary is 45,056 B. The default
Scoop kiennq shim is 136,192 B and uses statically linked MSVC C. Scoop also
added and then removed a 317,952 B Rust shim.

### Build Constraints

1. **`atexit` link failure**: Current nightly toolchains register TLS cleanup
   through C `atexit`. With `#![no_main]`, that symbol links
   `msvcrt.lib(utility.obj)`. The link then fails on undefined `__vcrt_*` and
   `__acrt_*` CRT initialization symbols. Export this no-op function:

   ```rust
   extern "C" fn atexit(...) -> i32 { 0 }
   ```

   See `src/win.rs`. The trampoline does not run TLS destructors at process exit.
   The documented `rustc-link-lib=ucrt` workaround does not fix this link. See
   rust-lang/rust#143172. The older nightly toolchain that uv uses does not
   register `atexit`.

2. **Subsystem**: `#![no_main]` needs
   `#![windows_subsystem = "console"]`. Without this attribute, lld reports
   that the subsystem is not defined.
3. **Static CRT**: Do not use `+crt-static`. It links the static CRT and
   increases the binary size to approximately 115 KiB.
4. **Development profile**: Use `opt-level = 1` and LTO. At `opt-level = 0`,
   the compiler can reference the MSVC helper `__CxxFrameHandler3`. This causes
   a link failure, even with `panic = "immediate-abort"`. uv uses the same
   settings.

### Remaining options

- Assign the child to a job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`.
  uv uses this option. It makes Windows stop the child when Windows stops the
  shim. It increases the binary size by a few KiB.
- Commit reproducible trampoline binaries. uv commits `/Brepro`-normalized
  executables and compares each byte in CI. This option isolates the shim from
  toolchain changes.

## References

- [Issue #835](https://github.com/voidzero-dev/vite-plus/issues/835): Original feature request with video reproduction
- [uv-trampoline](https://github.com/astral-sh/uv/tree/main/crates/uv-trampoline):
  Reference implementation by astral-sh. It uses workspace exclusion,
  build-std, `panic="immediate-abort"`, cargo-xwin, `#![no_main]`, and raw
  Win32. Its CI rejects `core::fmt` and `std::panicking` symbols.
- [Scoop shims](https://github.com/ScoopInstaller/Scoop/tree/master/supporting/shims):
  Native C shim from kiennq/scoop-better-shimexe and C# .NET shim. The C shim
  is 136 KiB. The C# shim is 9.7 KiB. A sibling `.shim` file specifies the
  launch target.
- [RFC: env-command](./env-command.md): Shim architecture documentation
- [RFC: upgrade-command](./upgrade-command.md): Upgrade/rollback flow
