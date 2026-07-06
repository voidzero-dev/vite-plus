# run_ctrlc_waits_for_task_shutdown

Issue #2036: Ctrl+C on `vp run dev` must let the task finish its graceful
shutdown before vp exits and the shell takes the terminal back. The verdict
below flips to "task was torn down before its graceful shutdown finished"
when vp abandons the task mid-shutdown again.

## `vp run dev`

**Exit code:** 1

**→ expect-milestone:** `ready`

```
VITE+ - The Unified Toolchain for the Web

$ vpt report-orphan-on-ctrlc verdict.txt ⊘ cache disabled
```

**← write-key:** `ctrl-c`

```
VITE+ - The Unified Toolchain for the Web

$ vpt report-orphan-on-ctrlc verdict.txt ⊘ cache disabled

```

## `vpt wait-file verdict.txt 15000`

The verdict recorded by the task's watcher process.

```
task completed its graceful shutdown
```
