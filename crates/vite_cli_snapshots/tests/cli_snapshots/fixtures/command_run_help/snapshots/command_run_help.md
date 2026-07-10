# command_run_help

## `vp run --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp run [OPTIONS] [TASK_SPECIFIER] [ADDITIONAL_ARGS]...

Run tasks.

Arguments:
  [TASK_SPECIFIER]      `packageName#taskName` or `taskName`. If omitted, lists all available tasks
  [ADDITIONAL_ARGS]...  Additional arguments to pass to the tasks

Options:
  -r, --recursive          Select all packages in the workspace
  -t, --transitive         Select the current package and its transitive dependencies
  -w, --workspace-root     Select the workspace root package
  -F, --filter <FILTERS>   Match packages by name, directory, or glob pattern
  --fail-if-no-match       Exit with a non-zero status if a filter matches no packages
  --ignore-depends-on      Do not run dependencies specified in `dependsOn` fields
  -v, --verbose            Show full detailed summary after execution
  --cache                  Force caching on for all tasks and scripts
  --no-cache               Force caching off for all tasks and scripts
  --log <MODE>             Set output mode: interleaved, labeled, or grouped
  --concurrency-limit <N>  Maximum number of tasks to run concurrently (default: 4)
  --parallel               Run tasks without dependency ordering (unlimited concurrency by default)
  --last-details           Display the detailed summary of the last run
  -h, --help               Print help (see more with '--help')

Filter Patterns:
  --filter <pattern>        Select by package name (e.g. foo, @scope/*)
  --filter ./<dir>          Select packages under a directory
  --filter {<dir>}          Same as ./<dir>, but allows traversal suffixes
  --filter <pattern>...     Select package and its dependencies
  --filter ...<pattern>     Select package and its dependents
  --filter <pattern>^...    Select only the dependencies (exclude the package itself)
  --filter !<pattern>       Exclude packages matching the pattern

Documentation: https://viteplus.dev/guide/run
```

## `vp help run`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp run [OPTIONS] [TASK_SPECIFIER] [ADDITIONAL_ARGS]...

Run tasks.

Arguments:
  [TASK_SPECIFIER]      `packageName#taskName` or `taskName`. If omitted, lists all available tasks
  [ADDITIONAL_ARGS]...  Additional arguments to pass to the tasks

Options:
  -r, --recursive          Select all packages in the workspace
  -t, --transitive         Select the current package and its transitive dependencies
  -w, --workspace-root     Select the workspace root package
  -F, --filter <FILTERS>   Match packages by name, directory, or glob pattern
  --fail-if-no-match       Exit with a non-zero status if a filter matches no packages
  --ignore-depends-on      Do not run dependencies specified in `dependsOn` fields
  -v, --verbose            Show full detailed summary after execution
  --cache                  Force caching on for all tasks and scripts
  --no-cache               Force caching off for all tasks and scripts
  --log <MODE>             Set output mode: interleaved, labeled, or grouped
  --concurrency-limit <N>  Maximum number of tasks to run concurrently (default: 4)
  --parallel               Run tasks without dependency ordering (unlimited concurrency by default)
  --last-details           Display the detailed summary of the last run
  -h, --help               Print help (see more with '--help')

Filter Patterns:
  --filter <pattern>        Select by package name (e.g. foo, @scope/*)
  --filter ./<dir>          Select packages under a directory
  --filter {<dir>}          Same as ./<dir>, but allows traversal suffixes
  --filter <pattern>...     Select package and its dependencies
  --filter ...<pattern>     Select package and its dependents
  --filter <pattern>^...    Select only the dependencies (exclude the package itself)
  --filter !<pattern>       Exclude packages matching the pattern

Documentation: https://viteplus.dev/guide/run
```
