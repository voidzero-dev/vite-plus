# command_vp_alias

## `vp -h`

vp should show help same as vite

```
VITE+ - The Unified Toolchain for the Web

Usage: vp <COMMAND>

Core Commands:
  create         Create a new project from a template
  migrate        Migrate an existing project to Vite+
  dev            Run the development server
  build          Build for production
  test           Run tests
  lint           Lint code
  fmt, format    Format code
  check          Run format, lint, and type checks
  pack           Build library
  run            Run tasks
  exec           Execute a command from local node_modules/.bin
  preview        Preview production build
  cache          Manage the task cache
  config         Configure hooks and agent integration
  staged         Run linters on staged files

Package Manager Commands:
  install    Install all dependencies, or add packages if package names are provided

Options:
  -h, --help  Print help
```

## `vp run -h`

vp run should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp run [OPTIONS] [TASK_SPECIFIER] [ADDITIONAL_ARGS]...

Run tasks.

Arguments:
  [TASK_SPECIFIER]      `packageName#taskName` or `taskName`. If omitted, shows the task selector
  [ADDITIONAL_ARGS]...  Additional arguments to pass to the task

Options:
  -r, --recursive          Select all packages in the workspace
  -t, --transitive         Select the current package and its transitive dependencies
  -w, --workspace-root     Select the workspace root package
  -F, --filter <FILTERS>   Match packages by name, directory, or glob pattern
  --fail-if-no-match       Exit with a non-zero status if a filter matches no packages
  --ignore-depends-on      Do not run dependencies specified in `dependsOn` fields
  -v, --verbose            Show the full detailed summary after execution
  --cache                  Force caching on for all tasks and scripts
  --no-cache               Force caching off for all tasks and scripts
  --log <MODE>             Set output mode: interleaved (default), labeled, or grouped
  --concurrency-limit <N>  Maximum number of tasks to run concurrently (default: 4)
  --parallel               Run tasks without dependency ordering; concurrency is unlimited unless `--concurrency-limit` is specified
  --last-details           Display the detailed summary of the last run
  -h, --help               Print help

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
