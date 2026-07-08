# ctrlc_interrupt

Scripts a ctrl-c: `vpt exit-on-ctrlc` emits a `ready` milestone, waits for the
interrupt, then prints and exits. This is the one scenario the runner isolates:
ctrl-c cases are auto-detected as signal-sensitive (see `case_needs_isolation`)
and take the exclusive execution lease, so parallel-PTY signal routing can't
perturb them while every other case still runs concurrently.

## `vpt exit-on-ctrlc`

**→ expect-milestone:** `ready`

```
```

**← write-key:** `ctrl-c`

```
ctrl-c received
```
