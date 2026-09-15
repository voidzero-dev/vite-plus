# command_update_yarn4_with_workspace

## `vp update testnpm2 --latest --filter app`

Berry filtered update fails instead of updating every workspace

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Berry `update`.
```

## `vp up -D --filter app`

the up alias rejects filters without a package argument

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Berry `update`.
```

## `vp update --filter *`

wildcard filters are also unsupported

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Berry `update`.
```

## `vp update --workspace --filter app @vite-plus-test/utils`

workspace dependency updates reject filters

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Berry `update`.
```

## `vp update testnpm2 --filter app --filter @vite-plus-test/utils --recursive`

recursive update must not silently discard workspace filters

**Exit code:** 1

```
Invalid argument: `--filter` is not supported by Yarn Berry `update`.
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

root, selected, and unselected workspace manifests remain unchanged

```
{
  "name": "command-update-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "dependencies": {
    "testnpm2": "*"
  },
  "packageManager": "yarn@4.10.3"
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "*",
    "testnpm2": "*"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "*"
  }
}
```

## `vpt stat-file yarn.lock --assert missing`

```
yarn.lock: missing
```

## `vpt stat-file .pnp.cjs --assert missing`

```
.pnp.cjs: missing
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vp update testnpm2`

unfiltered update still updates all testnpm2 versions

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + test-vite-plus-install@npm:1.0.0, test-vite-plus-package@npm:1.0.0, testnpm2@npm:1.0.1
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ 3 packages were added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json packages/utils/package.json`

```
{
  "name": "command-update-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "packageManager": "yarn@4.10.3"
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "*",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
{
  "name": "@vite-plus-test/utils",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": "^1.0.1"
  }
}
```

## `vp update -r --no-save`

unfiltered recursive update remains supported

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt print-file package.json packages/app/package.json`

```
{
  "name": "command-update-yarn4-with-workspace",
  "version": "1.0.0",
  "workspaces": [
    "packages/*"
  ],
  "dependencies": {
    "testnpm2": "^1.0.1"
  },
  "packageManager": "yarn@4.10.3"
}
{
  "name": "app",
  "dependencies": {
    "@vite-plus-test/utils": "workspace:*",
    "test-vite-plus-install": "*",
    "testnpm2": "^1.0.1"
  },
  "devDependencies": {
    "test-vite-plus-package": "*"
  }
}
```
