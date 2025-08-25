## fspy_preload_unix

The shared library injected by `DYLD_INSERT_LIBRARIES` on macOS and `LD_PRELOAD` on Linux to intercept file system calls.

This crates only contains code the shared library itself. The injection process is done in `fspy` crate.
