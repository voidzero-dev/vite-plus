# command_list_pnpm10

## `vp install`

should install packages first

```
VITE+ - The Unified Toolchain for the Web

dependencies:
 test-vite-plus-package-optional 1.0.0
 testnpm2 1.0.1

devDependencies:
 test-vite-plus-package 1.0.0

Done in <duration> using pnpm <version>
```

## `vp pm list --help`

should show help

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm list [OPTIONS] [PATTERN] [-- <PASS_THROUGH_ARGS>...]

List installed packages

Arguments:
  [PATTERN]               Package pattern to filter
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  --depth <DEPTH>          Maximum depth of dependency tree
  --json                   Output in JSON format
  --long                   Show extended information
  --parseable              Parseable output format
  -P, --prod               Only production dependencies
  -D, --dev                Only dev dependencies
  --no-optional            Exclude optional dependencies
  --exclude-peers          Exclude peer dependencies
  --only-projects          Show only project packages
  --find-by <FINDER_NAME>  Use a finder function
  -r, --recursive          List across all workspaces
  --filter <PATTERN>       Filter packages in monorepo
  -g, --global             List global packages
  -h, --help               Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp pm list`

should list installed packages

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
testnpm2 1.0.1

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp pm list testnpm2`

should list specific package

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1
```

## `vp pm list --depth 0`

should list packages with depth limit

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
testnpm2 1.0.1

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp pm list --json`

should list packages in JSON format

```
[
  {
    "name": "command-list-pnpm10",
    "version": "1.0.0",
    "path": "<workspace>",
    "private": false,
    "dependencies": {
      "test-vite-plus-package-optional": {
        "from": "test-vite-plus-package-optional",
        "version": "1.0.0",
        "resolved": "https://registry.npmjs.org/test-vite-plus-package-optional/-/test-vite-plus-package-optional-1.0.0.tgz",
        "path": "<workspace>/node_modules/.pnpm/test-vite-plus-package-optional@1.0.0/node_modules/test-vite-plus-package-optional"
      },
      "testnpm2": {
        "from": "testnpm2",
        "version": "1.0.1",
        "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz",
        "path": "<workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2"
      }
    },
    "devDependencies": {
      "test-vite-plus-package": {
        "from": "test-vite-plus-package",
        "version": "1.0.0",
        "resolved": "https://registry.npmjs.org/test-vite-plus-package/-/test-vite-plus-package-1.0.0.tgz",
        "path": "<workspace>/node_modules/.pnpm/test-vite-plus-package@1.0.0/node_modules/test-vite-plus-package"
      }
    }
  }
]
```

## `vp pm list --long`

should list packages with extended info

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
  just for snap-test
  <workspace>/node_modules/.pnpm/test-vite-plus-package-optional@1.0.0/node_modules/test-vite-plus-package-optional
testnpm2 1.0.1
  <workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2

devDependencies:
test-vite-plus-package 1.0.0
  just for snap-test
  <workspace>/node_modules/.pnpm/test-vite-plus-package@1.0.0/node_modules/test-vite-plus-package
```

## `vp pm list --parseable`

should list packages in parseable format

```
<workspace>
<workspace>/node_modules/.pnpm/test-vite-plus-package@1.0.0/node_modules/test-vite-plus-package
<workspace>/node_modules/.pnpm/test-vite-plus-package-optional@1.0.0/node_modules/test-vite-plus-package-optional
<workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2
```

## `vp pm list --prod`

should list production dependencies only

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
testnpm2 1.0.1
```

## `vp pm list --dev`

should list development dependencies only

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp pm list --no-optional`

should exclude optional dependencies

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
testnpm2 1.0.1

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp pm list --exclude-peers`

should exclude peer dependencies

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
testnpm2 1.0.1

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp pm list --only-projects`

should list only workspace projects (pnpm-specific)

```
```

## `vp pm list --find-by customFinder`

should use custom finder (pnpm-specific)

**Exit code:** 1

```
 ERR_PNPM_FINDER_NOT_FOUND  No finder with name customFinder is found
```

## `vp pm list --recursive`

should list packages recursively in workspace

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
testnpm2 1.0.1

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp pm list -- --loglevel=warn`

should support pass through arguments

```
Legend: production dependency, optional only, dev only

command-list-pnpm10@1.0.0 <workspace>

dependencies:
test-vite-plus-package-optional 1.0.0
testnpm2 1.0.1

devDependencies:
test-vite-plus-package 1.0.0
```
