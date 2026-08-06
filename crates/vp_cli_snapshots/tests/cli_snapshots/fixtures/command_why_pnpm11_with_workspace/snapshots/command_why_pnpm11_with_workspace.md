# command_why_pnpm11_with_workspace

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
testnpm2@1.0.0
└── command-why-pnpm11-with-workspace@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp why testnpm2 --filter app`

should check why in specific package

```
testnpm2@1.0.0
├── @vite-plus-test/utils (dependencies)
└── app (dependencies)

Found 1 version of testnpm2
```

## `vp why test-vite-plus-package -D --filter app`

should check why dev dependencies in app

```
test-vite-plus-package@1.0.0
└── app (devDependencies)

Found 1 version of test-vite-plus-package
```

## `vp why testnpm2 --filter *`

should check why in all packages

```
testnpm2@1.0.0
├── @vite-plus-test/utils (dependencies)
├── app (dependencies)
└── command-why-pnpm11-with-workspace@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp why testnpm2 -r`

should check why recursively

```
testnpm2@1.0.0
├── @vite-plus-test/utils (dependencies)
├── app (dependencies)
└── command-why-pnpm11-with-workspace@1.0.0 (dependencies)

Found 1 version of testnpm2
```

## `vp why testnpm2 --filter app --json`

should support json output with filter

```
[
  {
    "name": "testnpm2",
    "version": "1.0.0",
    "path": "<workspace>/node_modules/.pnpm/testnpm2@1.0.0/node_modules/testnpm2",
    "dependents": [
      {
        "name": "@vite-plus-test/utils",
        "version": "",
        "depField": "dependencies"
      },
      {
        "name": "app",
        "version": "",
        "depField": "dependencies"
      }
    ]
  }
]
```

## `vp why test-vite-plus-install --filter app --depth 1`

should support depth limiting with filter

```
test-vite-plus-install@1.0.0
└── app (dependencies)

Found 1 version of test-vite-plus-install
```
