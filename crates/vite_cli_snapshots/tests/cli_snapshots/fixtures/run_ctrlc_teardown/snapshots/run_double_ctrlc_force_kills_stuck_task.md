# run_double_ctrlc_force_kills_stuck_task

The escape hatch for the waiting behavior above: a task that ignores the
interrupt must not make vp unstoppable. On the second Ctrl+C vp warns,
kills its own process group (taking the whole task tree with it), and dies
with it, so only the watcher survives to record the unfinished shutdown.
The "interrupted" milestone between the two Ctrl+C presses keeps them from
coalescing into a single signal.

## `vp run stuck-dev`

**Exit code:** 1

**→ expect-milestone:** `ready`

```
VITE+ - The Unified Toolchain for the Web

$ vpt report-orphan-on-ctrlc verdict.txt --ignore-interrupt ⊘ cache disabled
```

**← write-key:** `ctrl-c`

**→ expect-milestone:** `interrupted`

```
VITE+ - The Unified Toolchain for the Web

$ vpt report-orphan-on-ctrlc verdict.txt --ignore-interrupt ⊘ cache disabled
ignoring interrupt; still running
```

**← write-key:** `ctrl-c`

```
VITE+ - The Unified Toolchain for the Web

$ vpt report-orphan-on-ctrlc verdict.txt --ignore-interrupt ⊘ cache disabled
ignoring interrupt; still running
warn: Force quitting the task
```

## `vpt wait-file verdict.txt 15000`

The verdict recorded by the task's watcher process.

```
task was torn down before its graceful shutdown finished
```
