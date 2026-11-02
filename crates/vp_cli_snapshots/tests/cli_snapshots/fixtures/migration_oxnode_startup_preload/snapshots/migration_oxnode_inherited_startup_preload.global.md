# migration_oxnode_inherited_startup_preload

Published oxc-node 1.0.1 verifies startup ordering with a preload inherited through NODE_OPTIONS and no register import to replace. The local registry supplies it; the JSX dependency is a local fixture.

## `vpt json-edit package.json scripts.run 'NODE_OPTIONS='\''-r ./setup.cjs'\'' oxnode entry.ts'`


## `vp install --ignore-scripts`


## `vp run run`

The inherited preload enables JSX dependency transforms before the standalone loader initializes.

```
VITE+ - The Unified Toolchain for the Web

$ NODE_OPTIONS='-r ./setup.cjs' oxnode entry.ts ⊘ cache disabled
dependency value: 42
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
  - Kept oxnode in package.json scripts "run": an additional startup preload can configure oxc-node hooks or dependency transforms; migrate it manually.
  - Kept the @oxc-node/cli / @oxc-node/core devDependency in package.json: `scripts` in package.json still uses oxnode.
```

## `vpt print-file package.json setup.cjs`

The build migrates, while the inherited-preload command and both oxc-node dependencies stay intact.

```
{
  "devDependencies": {
    "@oxc-node/cli": "1.0.1",
    "@oxc-node/core": "1.0.1",
    "jsx-dep": "file:.fixture/jsx-dep",
    "vite-plus": "catalog:"
  },
  "name": "migration-oxnode-startup-preload",
  "packageManager": "pnpm@12.11.2",
  "private": true,
  "scripts": {
    "build": "vpx build.ts",
    "run": "NODE_OPTIONS='-r ./setup.cjs' oxnode entry.ts"
  },
  "type": "module"
}
process.env.OXC_TRANSFORM_ALL = '1';
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
VITE+ - The Unified Toolchain for the Web

$ vpx build.ts ⊘ cache disabled
build runs
```

## `vp run run`

After a fresh install, the preserved command still prints the JSX dependency's value.

```
VITE+ - The Unified Toolchain for the Web

$ NODE_OPTIONS='-r ./setup.cjs' oxnode entry.ts ⊘ cache disabled
dependency value: 42
```
