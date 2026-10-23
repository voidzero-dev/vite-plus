# migration_oxnode_scripts

The oxc-node dependencies use local stub packages. Migrated scripts run the bundled loader.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

oxnode commands and shebangs migrate to vpx; preloads keep Node.js

```
VITE+ - The Unified Toolchain for the Web

● oxnode scripts detected. Auto-migrating to Vite+...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• oxnode scripts migrated to Vite+
```

## `vpt print-file package.json`

```
{
  "name": "migration-oxnode",
  "type": "module",
  "scripts": {
    "dev": "vpx --watch --clear-screen=false ./scripts/seed.ts",
    "seed": "vpx -C dev ./scripts/seed.ts",
    "loader": "node --import vite-plus/register ./scripts/seed.ts",
    "loader-eval": "node --import=vite-plus/register --eval \"import('./scripts/seed.ts')\"",
    "loader-test": "node --import vite-plus/register --test --test-reporter=dot ./scripts/seed.ts",
    "env-loader": "NODE_OPTIONS='--import vite-plus/register' node ./scripts/seed.ts"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file scripts/release.ts`

```
#!/usr/bin/env vpx
const version: string = '1.0.0';
console.log(version);
```

## `vp run seed`

the migrated command runs TypeScript with Node conditions

```
$ vpx -C dev ./scripts/seed.ts ⊘ cache disabled
ready
```

## `vp run loader`

the migrated Node loader command runs without standalone oxc-node

```
$ node --import vite-plus/register ./scripts/seed.ts ⊘ cache disabled
ready
```

## `vp run loader-eval`

the migrated preload keeps native Node.js eval behavior

```
$ node --import=vite-plus/register --eval "import('./scripts/seed.ts')" ⊘ cache disabled
ready
```

## `vp run loader-test`

```
$ node --import vite-plus/register --test --test-reporter=dot ./scripts/seed.ts ⊘ cache disabled
.
```
