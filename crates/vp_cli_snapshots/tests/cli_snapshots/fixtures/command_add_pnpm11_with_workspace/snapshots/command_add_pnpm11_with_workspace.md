# command_add_pnpm11_with_workspace

## `vp add testnpm2 -D -w`

should add package to workspace root

```

devDependencies:
 testnpm2 ^1.0.1

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-add-pnpm11-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
```

## `vp add @vite-plus-test/utils --workspace`

should add @vite-plus-test/utils to workspace root

```

dependencies:
 @vite-plus-test/utils workspace:*

Already up to date

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json`

```
{
  "name": "command-add-pnpm11-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*"
  }
}
```

## `vp add testnpm2 test-vite-plus-install@1.0.0 --filter app`

should add packages to packages/app

```
.                                        |   +1 +

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-pnpm11-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*"
  }
}
{
  "name": "app",
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add @vite-plus-test/utils --workspace --filter app`

should add @vite-plus-test/utils to packages/app

```

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-pnpm11-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true
}
```

## `vp add -E testnpm2 test-vite-plus-install --filter *`

should add testnpm2 test-vite-plus-install to all packages except workspace root

```

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-pnpm11-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
```

## `vp install test-vite-plus-package@1.0.0 --filter * --workspace-root --save-catalog`

should install packages alias for add command

```
VITE+ - The Unified Toolchain for the Web
.                                        |   +1 +

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json pnpm-workspace.yaml`

```
{
  "name": "command-add-pnpm11-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@11.0.6",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "catalog:"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "catalog:",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "catalog:",
    "testnpm2": "^1.0.1"
  }
}
packages:
  - packages/*
catalog:
  test-vite-plus-package: 1.0.0
```

## `vp add --filter app test-vite-plus-package-optional --save-catalog-name v1`

should add with save-catalog-name

```
.                                        |   +1 +

Done in <duration> using pnpm <version>
```

## `vpt print-file packages/app/package.json pnpm-workspace.yaml`

```
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "catalog:",
    "test-vite-plus-package-optional": "catalog:v1",
    "testnpm2": "^1.0.1"
  }
}
packages:
  - packages/*
catalog:
  test-vite-plus-package: 1.0.0
catalogs:
  v1:
    test-vite-plus-package-optional: ^1.0.0
```

## `vp add --filter=./packages/utils test-vite-plus-package-optional -O --save-catalog-name v2`

should add other with save-catalog-name

```

Done in <duration> using pnpm <version>
```

## `vpt print-file packages/utils/package.json pnpm-workspace.yaml`

```
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "catalog:",
    "testnpm2": "^1.0.1"
  },
  "optionalDependencies": {
    "test-vite-plus-package-optional": "catalog:v2"
  }
}
packages:
  - packages/*
catalog:
  test-vite-plus-package: 1.0.0
catalogs:
  v1:
    test-vite-plus-package-optional: ^1.0.0
  v2:
    test-vite-plus-package-optional: ^1.0.0
```
