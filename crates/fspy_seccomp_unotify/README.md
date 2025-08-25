# fspy_seccomp_unotify

Safe bindings for `seccomp_unotify` used by `fspy` to intercept direct syscalls on Linux.

- `src/supervisor` is gated by feature `supervisor`. It contains code that needs to run in the supervisor process(the process that uses `fspy` to track child processes).
- `src/target` is gated by feature `target`. It contains code that needs to run in target processes(child processes tracked by `fspy`).
