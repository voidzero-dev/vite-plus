# command_vpr

## `vpr -h`

should show vp run help

```
Run tasks

Usage: vp run [OPTIONS] [TASK_SPECIFIER] [ADDITIONAL_ARGS]...

Arguments:
  [TASK_SPECIFIER] [ADDITIONAL_ARGS]...
          Task to run, as `packageName#taskName` or just `taskName`.
          Any arguments after the task name are forwarded to the task process.
          Running `vp run` without a task name shows an interactive task selector.

Options:
  -r, --recursive
          Select all packages in the workspace
  -t, --transitive
          Select the current package and its transitive dependencies
  -w, --workspace-root
          Select the workspace root package
  -F, --filter <FILTERS>
          Match packages by name, directory, or glob pattern
      --fail-if-no-match
          Exit with a non-zero status if a `--filter` expression matches no packages
      --ignore-depends-on
          Do not run dependencies specified in `dependsOn` fields
  -v, --verbose
          Show full detailed summary after execution
      --cache
          Force caching on for all tasks and scripts
      --no-cache
          Force caching off for all tasks and scripts
      --log <LOG>
          How task output is displayed [default: interleaved] [possible values: interleaved, labeled, grouped]
      --concurrency-limit <CONCURRENCY_LIMIT>
          Maximum number of tasks to run concurrently. Defaults to 4
      --parallel
          Run tasks without dependency ordering. Sets concurrency to unlimited unless `--concurrency-limit` is also specified
      --last-details
          Display the detailed summary of the last run
  -h, --help
          Print help (see more with '--help')
```

## `vpr hello`

should run script via vpr shorthand

```
$ node args.mjs hello from script ⊘ cache disabled
hello
from
script
```

## `vpr greet --arg1 value1`

should pass through additional args

```
$ node args.mjs greet --arg1 value1 ⊘ cache disabled
greet
--arg1
value1
```

## `vpr nonexistent`

should show pnpm missing script error

**Exit code:** 1

```
Task "nonexistent" not found.
```
