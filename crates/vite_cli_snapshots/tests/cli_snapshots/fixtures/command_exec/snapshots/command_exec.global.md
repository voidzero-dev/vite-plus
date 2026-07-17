# command_exec

## `node setup-bin.js`


## `vp exec hello-test`

exec binary from node_modules/.bin

```
VITE+ - The Unified Toolchain for the Web

hello from test-bin
```

## `vp exec echo hello`

basic exec

```
VITE+ - The Unified Toolchain for the Web

hello
```

## `vp exec -- echo with-separator`

explicit -- separator

```
VITE+ - The Unified Toolchain for the Web

with-separator
```

## `vp exec node -e 'console.log('\''from node'\'')'`

exec node with args

```
VITE+ - The Unified Toolchain for the Web

from node
```

## `vp exec -c 'echo hello from shell'`

shell mode

```
VITE+ - The Unified Toolchain for the Web

hello from shell
```

## `vp exec --parallel -- echo hello`

--parallel with single package should stream output

```
VITE+ - The Unified Toolchain for the Web

hello
```

## `cd subdir && vp exec ./my-local`

resolve relative executable from caller cwd

```
VITE+ - The Unified Toolchain for the Web

hello from subdir
```

## `vp exec --help`

help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp exec [OPTIONS] [COMMAND]...

Execute a command from local node_modules/.bin.

Arguments:
  [COMMAND]...  Command and arguments to execute

Options:
  -r, --recursive              Select all packages in the workspace
  -t, --transitive             Select the current package and its transitive dependencies
  -w, --workspace-root         Select the workspace root package
  -F, --filter <FILTERS>       Match packages by name, directory, or glob pattern
  --fail-if-no-match           Exit with a non-zero status if a filter matches no packages
  -c, --shell-mode             Execute the command within a shell environment
  --parallel                   Run concurrently without topological ordering
  --reverse                    Reverse execution order
  --resume-from <RESUME_FROM>  Resume from a specific package
  --report-summary             Save results to vp-exec-summary.json
  -h, --help                   Print help (see more with '--help')

Filter Patterns:
  --filter <pattern>        Select by package name (e.g. foo, @scope/*)
  --filter ./<dir>          Select packages under a directory
  --filter {<dir>}          Same as ./<dir>, but allows traversal suffixes
  --filter <pattern>...     Select package and its dependencies
  --filter ...<pattern>     Select package and its dependents
  --filter <pattern>^...    Select only the dependencies (exclude the package itself)
  --filter !<pattern>       Exclude packages matching the pattern

Examples:
  vp exec node --version                             # Run local node
  vp exec tsc --noEmit                               # Run local TypeScript compiler
  vp exec -c 'tsc --noEmit && prettier --check .'    # Shell mode
  vp exec -r -- tsc --noEmit                         # Run in all workspace packages
  vp exec --filter 'app...' -- tsc                   # Run in filtered packages

Documentation: https://viteplus.dev/guide/vpx
```

## `vp exec`

missing command should error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: 'vp exec' requires a command to run

Usage: vp exec [--] <command> [args...]

Examples:
  vp exec node --version
  vp exec tsc --noEmit
```

## `vp exec nonexistent-cmd-12345`

command not found error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Command 'nonexistent-cmd-12345' not found in node_modules/.bin

Run `vp install` to install dependencies, or use `vpx` for invoking remote commands.
```

## `vp run foo`

vp exec works in package.json scripts

```
VITE+ - The Unified Toolchain for the Web

$ vp exec node -e "console.log(5173)" ⊘ cache disabled
VITE+ - The Unified Toolchain for the Web

5173
```
