# run_double_ctrlc_force_kills_stuck_task

The escape hatch for the waiting behavior above: a task that ignores the
interrupt must not make vp unstoppable. The second Ctrl+C force-kills the
task, so vp exits with the kill signal's code and the watcher records an
unfinished shutdown. The "interrupted" milestone between the two Ctrl+C
presses keeps them from coalescing into a single signal.

## `vp run stuck-dev`

**Exit code:** 137

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

```

## `vpt wait-file verdict.txt`

The verdict recorded by the task's watcher process.

```
task was torn down before its graceful shutdown finished
```
