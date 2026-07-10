# command_why_pnpm10

## `vp why --help`

should show help

```
Show why a package is installed

Usage: vp why [OPTIONS] <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

Arguments:
  <PACKAGES>...           Package(s) to check
  [PASS_THROUGH_ARGS]...  Additional arguments to pass through to the package manager

Options:
      --json                   Output in JSON format
      --long                   Show extended information
      --parseable              Show parseable output
  -r, --recursive              Check recursively across all workspaces
      --filter <PATTERN>       Filter packages in monorepo
  -w, --workspace-root         Check in workspace root
  -P, --prod                   Only production dependencies
  -D, --dev                    Only dev dependencies
      --depth <DEPTH>          Limit tree depth
      --no-optional            Exclude optional dependencies
      --exclude-peers          Exclude peer dependencies
      --find-by <FINDER_NAME>  Use a finder function defined in .pnpmfile.cjs
  -h, --help                   Print help
```

## `vp install`

should install packages first

```

dependencies:
 testnpm2 1.0.1

optionalDependencies:
 test-vite-plus-package-optional 1.0.0

devDependencies:
 test-vite-plus-package 1.0.0

Done in <duration> using pnpm <version>
```

## `vp why testnpm2`

should show why package is installed

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1
```

## `vp explain testnpm2`

should work with explain alias

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1
```

## `vp why test-vite-plus-package`

should show why dev package is installed

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp why testnpm2 test-vite-plus-package`

should support multiple packages

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp why testnpm2 --json`

should support json output

```
[
  {
    "name": "command-why-pnpm10",
    "version": "1.0.0",
    "path": "<workspace>",
    "private": false,
    "dependencies": {
      "testnpm2": {
        "from": "testnpm2",
        "version": "1.0.1",
        "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.1.tgz",
        "path": "<workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2"
      }
    }
  }
]
```

## `vp why testnpm2 --long`

should support long output

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1
  <workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2
```

## `vp why testnpm2 --parseable`

should support parseable output

```
<workspace>
<workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2
```

## `vp why testnpm2 -P`

should support prod dependencies only

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1
```

## `vp why test-vite-plus-package -D`

should support dev dependencies only

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp why testnpm2 --depth 1`

should support depth limiting

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1
```

## `vp why test-vite-plus-package-optional --no-optional`

should exclude optional dependencies

```
```

## `vp why testnpm2 --find-by customFinder`

should support find-by option (pnpm-specific)

**Exit code:** 1

```
 ERR_PNPM_FINDER_NOT_FOUND  No finder with name customFinder is found
```

## `vp why testnpm2 -- --reporter=silent`

should support pass through arguments

```
Legend: production dependency, optional only, dev only

command-why-pnpm10@1.0.0 <workspace>

dependencies:
testnpm2 1.0.1
```
