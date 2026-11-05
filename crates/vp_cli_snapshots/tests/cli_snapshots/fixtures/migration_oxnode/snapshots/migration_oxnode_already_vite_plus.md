# migration_oxnode_already_vite_plus

## `vpt json-edit package.json devDependencies.vite-plus latest`


## `vp migrate --no-interactive`

a plain upgrade leaves oxnode migration to --full

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
• Skipped editor, hooks, and lint setup. Run `vp migrate --full` to apply them.
```

## `vp migrate --no-interactive --full --no-hooks --no-agent --no-editor`

--full migrates oxnode on an existing Vite+ project

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● oxnode scripts detected. Auto-migrating to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• oxnode scripts migrated to Vite+
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-oxnode",
  "scripts": {
    "dev": "vpx --watch --clear-screen=false ./scripts/seed.ts",
    "env-loader": "NODE_OPTIONS='--import vite-plus/register' node ./scripts/seed.ts",
    "loader": "node --import vite-plus/register ./scripts/seed.ts",
    "loader-eval": "node --import=vite-plus/register --eval \"import('./scripts/seed.ts')\"",
    "loader-test": "node --import vite-plus/register --test --test-reporter=dot ./scripts/seed.ts",
    "seed": "vpx -C dev ./scripts/seed.ts"
  },
  "type": "module"
}
```

## `vp run seed`

```
$ vpx -C dev ./scripts/seed.ts ⊘ cache disabled
ready
```
