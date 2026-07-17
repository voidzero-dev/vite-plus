# command_why_pnpm11

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
testnpm2@1.0.1
└── command-why-pnpm11@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp explain testnpm2`

should work with explain alias

```
testnpm2@1.0.1
└── command-why-pnpm11@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp why test-vite-plus-package`

should show why dev package is installed

```
test-vite-plus-package@1.0.0
└── command-why-pnpm11@1.0.0 (devDependencies)

Found 1 version of test-vite-plus-package
```

## `vp why testnpm2 test-vite-plus-package`

should support multiple packages

```
test-vite-plus-package@1.0.0
└── command-why-pnpm11@1.0.0 (devDependencies)

testnpm2@1.0.1
└── command-why-pnpm11@1.0.0 (dependencies)

Found 1 version of test-vite-plus-package
Found 1 version of testnpm2
```

## `vp why testnpm2 --json`

should support json output

```
[
  {
    "name": "testnpm2",
    "version": "1.0.1",
    "path": "<workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2",
    "dependents": [
      {
        "name": "command-why-pnpm11",
        "version": "1.0.0",
        "depField": "dependencies"
      }
    ]
  }
]
```

## `vp why testnpm2 --long`

should support long output

```
testnpm2@1.0.1
│ <workspace>/node_modules/.pnpm/testnpm2@1.0.1/node_modules/testnpm2
└── command-why-pnpm11@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp why testnpm2 --parseable`

should support parseable output

```
command-why-pnpm11@1.0.0 > testnpm2@1.0.1
```

## `vp why testnpm2 -P`

should support prod dependencies only

```
testnpm2@1.0.1
└── command-why-pnpm11@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp why test-vite-plus-package -D`

should support dev dependencies only

```
test-vite-plus-package@1.0.0
└── command-why-pnpm11@1.0.0 (devDependencies)

Found 1 version of test-vite-plus-package
```

## `vp why testnpm2 --depth 1`

should support depth limiting

```
testnpm2@1.0.1
└── command-why-pnpm11@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp why test-vite-plus-package-optional --no-optional`

should exclude optional dependencies

```
```

## `vp why testnpm2 --find-by customFinder`

should support find-by option (pnpm-specific)

**Exit code:** 1

```
[ERR_PNPM_FINDER_NOT_FOUND] No finder with name customFinder is found
```

## `vp why testnpm2 -- --reporter=silent`

should support pass through arguments

```
testnpm2@1.0.1
└── command-why-pnpm11@1.0.0 (dependencies)

Found 1 version of testnpm2
```
