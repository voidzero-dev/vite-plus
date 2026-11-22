# migration_oxnode_env_file

Local oxc-node stub packages show whether Node can still load the standalone registration named only in explicitly selected environment files.

## `vp install --ignore-scripts`


## `vp run run`

```
$ node --env-file=config/node.env run.mjs ⊘ cache disabled
standalone registration loaded
application ran
```

## `vp run run-optional`

```
$ node --env-file-if-exists config/runtime.conf run.mjs ⊘ cache disabled
standalone registration loaded
application ran
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● oxnode scripts detected. Auto-migrating to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• oxnode scripts migrated to Vite+
• Package manager settings configured
! Warnings:
  - Kept the @oxc-node/cli / @oxc-node/core devDependency in package.json: config/node.env still uses oxnode.
```

## `vpt print-file package.json config/node.env config/runtime.conf`

The build migrates, while the env-file commands and their required loader dependency stay intact.

```
{
  "name": "migration-oxnode-env-file",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vpx build.ts",
    "run": "node --env-file=config/node.env run.mjs",
    "run-optional": "node --env-file-if-exists config/runtime.conf run.mjs"
  },
  "devDependencies": {
    "@oxc-node/cli": "file:.fixture/oxc-cli",
    "@oxc-node/core": "file:.fixture/oxc-core",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@12.11.2"
}
NODE_OPTIONS=--import=@oxc-node/core/register
NODE_OPTIONS=--import=@oxc-node/core/register
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vpt stat-file node_modules/@oxc-node/cli node_modules/@oxc-node/core`

```
node_modules/@oxc-node/cli: symlink
node_modules/@oxc-node/core: symlink
```

## `vp run build`

```
$ vpx build.ts ⊘ cache disabled
build runs
```

## `vp run run`

The required env file still loads the registration after a fresh install.

```
$ node --env-file=config/node.env run.mjs ⊘ cache disabled
standalone registration loaded
application ran
```

## `vp run run-optional`

The optional env file also keeps its registration dependency available.

```
$ node --env-file-if-exists config/runtime.conf run.mjs ⊘ cache disabled
standalone registration loaded
application ran
```
