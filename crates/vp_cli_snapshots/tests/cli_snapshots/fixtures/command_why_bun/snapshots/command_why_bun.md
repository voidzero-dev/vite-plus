# command_why_bun

## `vp why --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp why [OPTIONS] <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

Show why a package is installed

Arguments:
  <PACKAGES>...           Package(s) to check
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
  --json                   Output in JSON format
  --long                   Show extended information
  --parseable              Show parseable output
  -r, --recursive          Check recursively across all workspaces
  --filter <PATTERN>       Filter packages in monorepo
  -w, --workspace-root     Check in workspace root
  -P, --prod               Only production dependencies
  -D, --dev                Only dev dependencies
  --depth <DEPTH>          Limit tree depth
  --no-optional            Exclude optional dependencies
  --exclude-peers          Exclude peer dependencies
  --find-by <FINDER_NAME>  Use a finder function defined in .pnpmfile.cjs
  -h, --help               Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp install`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

bun install <version> (af24e281)

 test-vite-plus-package@1.0.0
 test-vite-plus-package-optional@1.0.0
 testnpm2@1.0.1

3 packages installed [<duration>]
```

## `vp why testnpm2`

should show why package is installed

```
testnpm2@1.0.1
  └─ command-why-bun (requires 1.0.1)
```
