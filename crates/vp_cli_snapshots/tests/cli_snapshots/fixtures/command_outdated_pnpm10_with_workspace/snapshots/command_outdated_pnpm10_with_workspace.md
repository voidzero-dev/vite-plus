# command_outdated_pnpm10_with_workspace

## `vp install`

```
VITE+ - The Unified Toolchain for the Web

Scope: all 3 workspace projects

dependencies:
 testnpm2 1.0.0 (1.0.1 is available)

Done in <duration> using pnpm <version>
```

## `vp outdated testnpm2 -w`

should outdated in workspace root

**Exit code:** 1

```
┌──────────┬─────────┬────────┐
│ Package  │ Current │ Latest │
├──────────┼─────────┼────────┤
│ testnpm2 │ 1.0.0   │ 1.0.1  │
└──────────┴─────────┴────────┘
```

## `vp outdated testnpm2 --filter app`

should outdated in specific package

**Exit code:** 1

```
┌──────────┬─────────┬────────┬────────────┐
│ Package  │ Current │ Latest │ Dependents │
├──────────┼─────────┼────────┼────────────┤
│ testnpm2 │ 1.0.0   │ 1.0.1  │ app        │
└──────────┴─────────┴────────┴────────────┘
```

## `vp outdated -D --filter app`

should outdated dev dependencies in app

```
```

## `vp outdated --filter * --format json`

should outdated in all packages

**Exit code:** 1

```
{
  "testnpm2": {
    "current": "1.0.0",
    "latest": "1.0.1",
    "wanted": "1.0.0",
    "isDeprecated": false,
    "dependencyType": "dependencies",
    "dependentPackages": [
      {
        "name": "command-outdated-pnpm10-with-workspace",
        "location": "<workspace>"
      },
      {
        "name": "app",
        "location": "<workspace>/packages/app"
      },
      {
        "name": "@vite-plus-test/utils",
        "location": "<workspace>/packages/utils"
      }
    ]
  },
  "test-vite-plus-other-optional": {
    "current": "1.0.0",
    "latest": "1.1.0",
    "wanted": "1.0.0",
    "isDeprecated": false,
    "dependencyType": "optionalDependencies",
    "dependentPackages": [
      {
        "name": "app",
        "location": "<workspace>/packages/app"
      }
    ]
  }
}
```

## `vp outdated -r`

should outdated recursively

**Exit code:** 1

```
┌──────────────────────────────────────────┬─────────┬────────┬────────────────────────────────┐
│ Package                                  │ Current │ Latest │ Dependents                     │
├──────────────────────────────────────────┼─────────┼────────┼────────────────────────────────┤
│ testnpm2                                 │ 1.0.0   │ 1.0.1  │ @vite-plus-test/utils, app,    │
│                                          │         │        │ command-outdated-pnpm10-with-  │
│                                          │         │        │ workspace                      │
├──────────────────────────────────────────┼─────────┼────────┼────────────────────────────────┤
│ test-vite-plus-other-optional (optional) │ 1.0.0   │ 1.1.0  │ app                            │
└──────────────────────────────────────────┴─────────┴────────┴────────────────────────────────┘
```
