# command_add_pnpm9_with_workspace

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
  "name": "command-add-pnpm9-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@9.15.9",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
```

## `vp add @vite-plus-test/utils --workspace`

should add @vite-plus-test/utils to workspace root

**Exit code:** 1

```
 ERR_PNPM_ADDING_TO_ROOT  Running this command will add the dependency to the workspace root, which might not be what you want - if you really meant it, make it explicit by running this command again with the -w flag (or --workspace-root). If you don't want to see this warning anymore, you may set the ignore-workspace-root-check setting to true.
```

*(skipped 1 step(s) to the next line boundary: step failed)*

## `vp add testnpm2 test-vite-plus-install@1.0.0 --filter app`

should add packages to packages/app

```
.                                        |  WARN  `node_modules` is present. Lockfile only installation will make it out-of-date
.                                        |   +1 +

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-pnpm9-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@9.15.9",
  "devDependencies": {
    "testnpm2": "^1.0.1"
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
.                                        |  WARN  `node_modules` is present. Lockfile only installation will make it out-of-date

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-add-pnpm9-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@9.15.9",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
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
  "name": "command-add-pnpm9-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@9.15.9",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
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

## `vp install test-vite-plus-package@1.0.0 --filter * --workspace-root`

should install packages alias for add command

```
VITE+ - The Unified Toolchain for the Web
.                                        |   +1 +

Done in <duration> using pnpm <version>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json pnpm-workspace.yaml`

```
{
  "name": "command-add-pnpm9-with-workspace",
  "version": "1.0.0",
  "packageManager": "pnpm@9.15.9",
  "devDependencies": {
    "testnpm2": "^1.0.1"
  },
  "dependencies": {
    "test-vite-plus-package": "1.0.0"
  }
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:^",
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "private": true,
  "dependencies": {
    "test-vite-plus-install": "1.0.0",
    "test-vite-plus-package": "1.0.0",
    "testnpm2": "^1.0.1"
  }
}
packages:
  - packages/*
```

## `vp add --filter app test-vite-plus-package-optional --save-catalog-name v1`

should error because save-catalog-name is not supported at pnpm@9

**Exit code:** 1

```
 ERROR  Unknown option: 'save-catalog-name'
Did you mean 'save-optional'? Use "--config.unknown=value" to force an unknown option.
For help, run: pnpm help add
```

## `vp add --filter=./packages/utils test-vite-plus-package-optional -O --save-catalog v2`

should error because save-catalog is not supported at pnpm@9

**Exit code:** 1

```
 ERROR  Unknown option: 'save-catalog-name'
Did you mean 'save-optional'? Use "--config.unknown=value" to force an unknown option.
For help, run: pnpm help add
```
