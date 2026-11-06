# migration_oxnode_startup_preload

Published oxc-node 1.0.1 is needed to compare its startup ordering before and after migration. The local registry supplies it; the JSX dependency is a local fixture.

## `vp install --ignore-scripts`


## `vp run run`

The startup preload enables JSX dependency transforms before the standalone loader initializes.

```
VITE+ - The Unified Toolchain for the Web

$ oxnode -r ./setup.cjs entry.ts ⊘ cache disabled
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

The ordinary build migrates, while the command with a startup preload and its dependencies stay intact.

```
{
  "name": "migration-oxnode-startup-preload",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vpx build.ts",
    "run": "oxnode -r ./setup.cjs entry.ts"
  },
  "devDependencies": {
    "@oxc-node/cli": "1.0.1",
    "@oxc-node/core": "1.0.1",
    "jsx-dep": "file:.fixture/jsx-dep",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@12.11.2"
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

The retained command still transforms its JSX dependency after a fresh install.

```
VITE+ - The Unified Toolchain for the Web

$ oxnode -r ./setup.cjs entry.ts ⊘ cache disabled
dependency value: 42
```
