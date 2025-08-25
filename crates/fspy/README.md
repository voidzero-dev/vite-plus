# fspy

Run a command and capture all the paths it tries to access.

## macOS/Linux implementation

It uses `DYLD_INSERT_LIBRARIES` on macOS and `LD_PRELOAD` on Linux to inject a shared library that intercepts file system calls.
The injection process is almost identical on both platforms other than the environment variable name. The implementation is in `src/unix`.

## Linux-specific implementation for fully static binaries

For fully static binaries (such as `esbuild`), `LD_PRELOAD` does not work. In this case, `seccomp_unotify` is used to intercept direct system calls. The handler is implemented in `src/unix/syscall_handler`.

## Windows implementation

It uses [Detours](https://github.com/microsoft/Detours) to intercept file system calls. The implementation is in `src/windows`.

## Unified interface
The unified interface of `Command` is in `src/command.rs`.


## Preload Libraries

`DYLD_INSERT_LIBRARIES`, `LD_PRELOAD`, `Detours` all require a shared library to be injected. The shared libraries of macOS/Linux are in the `fspy_preload_unix` crate, and the shared library of Windows is in the `fspy_preload_windows` crate.
