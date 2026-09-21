# bun_frozen_lockfile

## `vpt json-edit package.json packageManager bun@1.3.11`


## `vp install ./dep --lockfile-only`

```
VITE+ - The Unified Toolchain for the Web

bun add <version> (<hash>)

Saved bun.lock (2 packages) [<duration>]
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vpt cp bun.lock before.lock`


## `vp install ./dep-v2 --frozen-lockfile`

a named-package install must not rewrite a frozen lockfile

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

bun add <version> (<hash>)
error: lockfile had changes, but lockfile is frozen
note: try re-running without --frozen-lockfile and commit the updated lockfile
```

## `node assert-lockfile-unchanged.mjs bun.lock`

```
lockfile unchanged
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vpt print-file package.json`

```
{
  "name": "install-package-options",
  "packageManager": "bun@1.3.11",
  "private": true,
  "version": "1.0.0",
  "dependencies": {
    "install-option-dep": "./dep"
  }
}
```
