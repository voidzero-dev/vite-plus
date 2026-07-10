# command_why_pnpm10_with_workspace

## `vp install`

```
VITE+ - The Unified Toolchain for the Web

Scope: all 3 workspace projects

dependencies:
 testnpm2 1.0.0 (1.0.1 is available)

Done in <duration> using pnpm <version>
```

## `vp why testnpm2 -w`

should check why in workspace root

```
Legend: production dependency, optional only, dev only

command-why-pnpm10-with-workspace@1.0.0 <workspace>

dependencies:
testnpm2 1.0.0
```

## `vp why testnpm2 --filter app`

should check why in specific package

```
Legend: production dependency, optional only, dev only

app <workspace>/packages/app

dependencies:
@vite-plus-test/utils link:../utils
└── testnpm2 1.0.0
testnpm2 1.0.0
```

## `vp why test-vite-plus-package -D --filter app`

should check why dev dependencies in app

```
Legend: production dependency, optional only, dev only

app <workspace>/packages/app

devDependencies:
test-vite-plus-package 1.0.0
```

## `vp why testnpm2 --filter *`

should check why in all packages

```
Legend: production dependency, optional only, dev only

command-why-pnpm10-with-workspace@1.0.0 <workspace>

dependencies:
testnpm2 1.0.0

app <workspace>/packages/app

dependencies:
@vite-plus-test/utils link:../utils
└── testnpm2 1.0.0
testnpm2 1.0.0

@vite-plus-test/utils <workspace>/packages/utils

dependencies:
testnpm2 1.0.0
```

## `vp why testnpm2 -r`

should check why recursively

```
Legend: production dependency, optional only, dev only

command-why-pnpm10-with-workspace@1.0.0 <workspace>

dependencies:
testnpm2 1.0.0

app <workspace>/packages/app

dependencies:
@vite-plus-test/utils link:../utils
└── testnpm2 1.0.0
testnpm2 1.0.0

@vite-plus-test/utils <workspace>/packages/utils

dependencies:
testnpm2 1.0.0
```

## `vp why testnpm2 --filter app --json`

should support json output with filter

```
[
  {
    "name": "app",
    "path": "<workspace>/packages/app",
    "private": false,
    "dependencies": {
      "testnpm2": {
        "from": "testnpm2",
        "version": "1.0.0",
        "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.0.tgz",
        "path": "<workspace>/node_modules/.pnpm/testnpm2@1.0.0/node_modules/testnpm2"
      },
      "@vite-plus-test/utils": {
        "from": "@vite-plus-test/utils",
        "version": "link:../utils",
        "path": "<workspace>/packages/utils",
        "dependencies": {
          "testnpm2": {
            "from": "testnpm2",
            "version": "1.0.0",
            "resolved": "https://registry.npmjs.org/testnpm2/-/testnpm2-1.0.0.tgz",
            "path": "<workspace>/node_modules/.pnpm/testnpm2@1.0.0/node_modules/testnpm2"
          }
        }
      }
    }
  }
]
```

## `vp why test-vite-plus-install --filter app --depth 1`

should support depth limiting with filter

```
Legend: production dependency, optional only, dev only

app <workspace>/packages/app

dependencies:
test-vite-plus-install 1.0.0
```
