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
├── Cargo.toml           # Zero dependencies, own release profile
├── Cargo.lock           # Own lockfile (the crate is not a workspace member)
├── .cargo/
│   └── config.toml      # build-std flags + target-dir = repo-root target/
├── src/
│   ├── main.rs          # Entry points + portable non-Windows fallback
│   ├── win.rs           # Windows implementation: raw Win32, no_main entry
│   └── cmdline.rs       # Command-line and sidecar parsers with portable tests
```

The crate is excluded from the workspace (`exclude` in the root `Cargo.toml`).
Two build requirements force this:

- The release profile sets `panic = "immediate-abort"`. Cargo ignores `panic`
  in per-package profile overrides, so the crate needs its own profile.
- The crate-local `.cargo/config.toml` enables build-std. Cargo reads that
  config only when it runs from the crate directory.

Build it from the crate directory:

```bash
cd crates/vp_trampoline && cargo build --release [--target <triple>]
```

Artifacts land in the repo-root `target/` directory (the crate config sets
`target-dir = "../../target"`), so CI steps and `install-global-cli` find
`vp-shim.exe` in the same place as workspace-built binaries. The build needs
the pinned nightly toolchain and the `rust-src` component; both come from the
repo `rust-toolchain.toml`.

### Trampoline Binary

The trampoline has **zero external dependencies**: all Win32 calls are raw
`extern "system"` declarations against KERNEL32, so the heavy
`windows`/`windows-core` crates never enter the build. It also never touches
`core::fmt`; diagnostics go through `WriteFile` with a hand-rolled decimal
formatter.

On Windows the binary is `#![no_main]` with an exported `mainCRTStartup`
symbol, so neither the CRT startup nor `std` runtime init runs. The flow in
`src/win.rs`:

1. `GetModuleFileNameW` gives the shim path and tool name. Replacing the `.exe`
   extension with `.shim` locates the per-tool sidecar.
2. `CreateFileW` and `ReadFile` load the UTF-8 sidecar. The parser requires the
   versioned header and accepts the `single-root` and `split` layouts.
3. `SetEnvironmentVariableW` pins the sidecar's layout. A single-root pointer
   sets `VP_HOME`. A split pointer removes `VP_HOME` and sets `VP_DATA_DIR`,
   `VP_BIN_DIR`, and `VP_CACHE_DIR`. Tool shims also set `VP_SHIM_TOOL` and
   remove `VP_TOOL_RECURSION`.
4. The child command line is `"<DATA>\current\bin\vp.exe"` plus the raw
   `GetCommandLineW` tail after the program argument. The split follows the
   MSVC `argv[0]` rule: quotes toggle, and backslashes do not escape. This
   preserves the caller's exact UTF-16 argument tail.
5. `SetConsoleCtrlHandler` installs a handler that ignores Ctrl+C and
   Ctrl+Break; the child decides how to react.
6. `CreateProcessW` spawns the child with inherited handles and startup info.
   When the parent redirected stdio (`STARTF_USESTDHANDLES`), the standard
   handles are forced inheritable first, as in uv-trampoline and distlib.
7. `WaitForSingleObject`, `GetExitCodeProcess`, and `ExitProcess` propagate the
   child's exit code unchanged.

Launch-critical failures report the failed call or operation, the relevant
path, and the Windows error code when one exists. A missing `vp.exe` also prints
a recovery hint to reinstall Vite+ or run `vp env setup`.

The non-Windows implementation uses `std::process::Command` and the same
sidecar parser for portable tests. Unix shims are symlinks and never use it.
The parser rejects missing, malformed, and unversioned sidecars. It does not
infer the layout from directory paths.

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

**Binary size**: 13,312 B on x86_64-pc-windows-msvc and 13,824 B on
aarch64-pc-windows-msvc, including sidecar parsing and error diagnostics. The
sidecar-aware `std::process::Command` implementation was 221,696 B on x86_64.
See Future Optimizations for the measured size ladder. The executable imports
only KERNEL32.

### Environment Variables

The sidecar controls the directory environment inherited by `vp.exe`:

| Variable            | When                    | Purpose                                               |
| ------------------- | ----------------------- | ----------------------------------------------------- |
| `VP_HOME`           | Single-root layout      | Pins all Vite+ directories to the sidecar's data root |
| `VP_HOME`           | Split layout            | Removed so it cannot override the category roots      |
| `VP_DATA_DIR`       | Split layout            | Pins the payload and state root                       |
| `VP_BIN_DIR`        | Split layout            | Pins the directory that contains the shim             |
| `VP_CACHE_DIR`      | Split layout            | Pins the cache root                                   |
| `VP_SHIM_TOOL`      | Tool shims, except `vp` | Selects shim dispatch for the named tool              |
| `VP_TOOL_RECURSION` | Removed for tool shims  | Forces fresh version resolution for nested shim calls |

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
Trampoline (node.exe + node.shim)
  → loads the recorded directory layout
  → sets VP_SHIM_TOOL=node and the directory pins, removes VP_TOOL_RECURSION
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
| **Toolchain**       | Nightly Rust (`panic="immediate-abort"`) | Nightly Rust (same technique)     |
| **Binary size**     | 39-47 KiB                                | 13-14 KiB                         |
| **Entry point**     | `#![no_main]` + `mainCRTStartup`         | Same approach                     |
| **Error output**    | `ufmt` (no `core::fmt`)                  | `WriteFile` + Win32 error codes   |
| **Ctrl+C handling** | `SetConsoleCtrlHandler` → ignore         | Same approach                     |
| **Exit code**       | `GetExitCodeProcess` → `exit()`          | Same approach                     |

The Vite+ trampoline is smaller because it embeds no PE resources and needs no
path canonicalization, job objects, or GUI subsystem support. It reads a small
sidecar next to its own filename, resolves `vp.exe` under the recorded data
root, and starts it. Both projects share the same build recipe and entry-point
structure.

## Alternatives Considered

### 1. NTFS Hardlinks (Rejected)

Hardlinks resolve to physical file inodes, not through directory junctions. After `vp` upgrade re-points `current`, hardlinks in `bin/` still reference the old binary.

### 2. Windows Symbolic Links (Rejected)

Requires administrator privileges or Developer Mode. Not reliable for all users.

### 3. PowerShell `.ps1` Scripts (Rejected)

`where.exe` and `which` do not find `.ps1` files. Only works in PowerShell.

### 4. Copy `vp.exe` as Each Shim (Rejected)

~5-10MB per copy. The trampoline achieves the same result in less than 14 KiB.

### 5. `windows` Crate for FFI (Rejected)

Adds ~100KB to the binary for a single `SetConsoleCtrlHandler` call. Raw FFI declaration is sufficient.

## Future Optimizations

Every variant below was built with cargo-xwin and measured on
x86_64-pc-windows-msvc. The first two rows use the sidecar-aware `std`
implementation. The next five rows are earlier fixed-layout experiments. The
last row is the current sidecar-aware raw implementation.

| Variant                                                                      | Toolchain | Size      |
| ---------------------------------------------------------------------------- | --------- | --------- |
| Sidecar-aware `std::process::Command`, precompiled `std`                     | stable    | 221,696 B |
| Same source + build-std + `panic="immediate-abort"`                          | nightly   | 82,432 B  |
| Fixed-layout `std` source + `#![no_main]` + `mainCRTStartup` + `atexit` stub | nightly   | 69,632 B  |
| Raw Win32 rewrite, normal `main`, stable, no build-std                       | stable    | 105,984 B |
| Raw Win32 rewrite, normal `main` + build-std                                 | nightly   | 13,824 B  |
| Raw Win32 rewrite + `#![no_main]`, no diagnostics                            | nightly   | 6,656 B   |
| Fixed-layout raw Win32 + `#![no_main]` + full diagnostics                    | nightly   | 8,192 B   |
| Sidecar-aware raw Win32 + `#![no_main]` + full diagnostics (shipped)         | nightly   | 13,312 B  |

For comparison: uv-trampoline ships 45,056 B (x64 console), Scoop's default
kiennq shim is 136,192 B (statically linked MSVC C), and Scoop once vendored
and then reverted a 317,952 B Rust shim.

### Gotchas (all hit while measuring)

1. **`atexit` link failure**: current nightlies register TLS destructor
   cleanup through C `atexit`. Under `#![no_main]` that symbol pulls
   `msvcrt.lib(utility.obj)`, and the link fails with undefined `__vcrt_*` /
   `__acrt_*` CRT init internals. Fix: export a no-op
   `extern "C" fn atexit(...) -> i32 { 0 }` (see win.rs). The trampoline
   never needs exit-time TLS destructors. uv's documented
   `rustc-link-lib=ucrt` workaround (rust-lang/rust#143172) does not fix this
   pull; uv's pinned older nightly simply predates the `atexit` registration.
2. **Subsystem**: `#![no_main]` requires an explicit
   `#![windows_subsystem = "console"]`, or lld fails with "subsystem must be
   defined".
3. **Do not use `+crt-static`**: it links the static CRT and grows the binary
   to ~115KB.
4. **Dev profile**: at `opt-level = 0` the compiler can emit references to
   the MSVC unwinding helper `__CxxFrameHandler3` even with
   `panic = "immediate-abort"`, and the link fails. Keep `opt-level = 1` and
   LTO in the dev profile (uv does the same).

### Remaining options

- Assign the child to a job object with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`
  (as uv does), so a killed shim also kills its child. Costs a few KB.
- Commit prebuilt, reproducible trampoline binaries (uv checks in
  `/Brepro`-normalized exes and verifies them byte for byte in CI) to
  decouple the shim from toolchain drift.

## References

- [Issue #835](https://github.com/voidzero-dev/vite-plus/issues/835): Original feature request with video reproduction
- [uv-trampoline](https://github.com/astral-sh/uv/tree/main/crates/uv-trampoline): Reference implementation by astral-sh. Same build recipe (workspace exclusion, build-std, `panic="immediate-abort"`, cargo-xwin), plus `#![no_main]`, raw Win32, and a CI `cargo bloat` gate that rejects any `core::fmt`/`std::panicking` symbol.
- [Scoop shims](https://github.com/ScoopInstaller/Scoop/tree/master/supporting/shims): vendored native C shim (136KB, from kiennq/scoop-better-shimexe) and C# .NET shim (9.7KB); launch targets come from a sibling `.shim` text file.
- [RFC: env-command](./env-command.md): Shim architecture documentation
- [RFC: upgrade-command](./upgrade-command.md): Upgrade/rollback flow
