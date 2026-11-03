# migration_ts_node_already_vite_plus

## `vpt json-edit package.json devDependencies.vite-plus latest`


## `vp migrate --no-interactive`

a plain upgrade leaves ts-node migration to --full

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

--full migrates transpile-only scripts on an existing Vite+ project

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● ts-node scripts detected. Auto-migrating supported commands to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• ts-node scripts migrated to Vite+
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "typescript": "file:.fixture/typescript",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-ts-node",
  "scripts": {
    "env-loader": "NODE_OPTIONS='--import vite-plus/register' node ./scripts/seed.ts",
    "loader": "node --import vite-plus/register ./scripts/seed.ts",
    "register": "node --import=vite-plus/register ./scripts/seed.ts",
    "seed": "vpx ./scripts/seed.ts"
  },
  "type": "module"
}
```

## `vp run seed`

```
$ vpx ./scripts/seed.ts ⊘ cache disabled
ready
```
