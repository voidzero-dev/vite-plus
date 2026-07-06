# run_ctrlc_exits_before_task_shutdown

Issue #2036: on Ctrl+C, `vp run` tears the task down immediately instead of
waiting for its graceful shutdown, and whatever the dying task tree still
does to the terminal races the next shell prompt. verdict.txt records the
current buggy behavior; a fix should flip it to
"task completed its graceful shutdown".

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

## `vpt wait-file verdict.txt`

The verdict the task wrote after `vp run` already exited.

```
task was torn down before its graceful shutdown finished
```
