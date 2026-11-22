# migration_tsx_preload

Plain tsx preloads keep Node.js and its flags. The local tsx stub keeps installation offline.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

Migrate preload flags and literal NODE_OPTIONS assignments.

```
VITE+ - The Unified Toolchain for the Web

● tsx scripts detected. Auto-migrating to Vite+...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• tsx scripts migrated to Vite+
```

## `vpt print-file package.json`

All commands keep Node.js or the tool launcher, and tsx is removed.

```
{
  "name": "migration-tsx-preload",
  "type": "module",
  "scripts": {
    "seed": "node --import=vite-plus/register ./scripts/seed.ts",
    "eval": "node --import vite-plus/register --eval \"import('./scripts/seed.ts')\"",
    "test": "node --import vite-plus/register --test",
    "watch": "node --import=vite-plus/register --watch --test ./scripts/seed.ts",
    "tool": "cross-env NODE_OPTIONS=\"--trace-warnings --import vite-plus/register\" mocha",
    "config": "TSX_TSCONFIG_PATH=tsconfig.json node --import vite-plus/register ./scripts/seed.ts",
    "launcher": "NODE_OPTIONS=--import=vite-plus/register concurrently \"node ./scripts/seed.ts\" \"node --test\""
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vp run seed`

The migrated file command resolves the installed preload.

```
$ node --import=vite-plus/register ./scripts/seed.ts ⊘ cache disabled
TypeScript preload works
```

## `vp run eval`

The migrated eval command keeps native Node.js behavior.

```
$ node --import vite-plus/register --eval "import('./scripts/seed.ts')" ⊘ cache disabled
TypeScript preload works
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

A second migration finds no remaining tsx use.

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

The scripts are unchanged on the second run.

```
{
  "name": "migration-tsx-preload",
  "type": "module",
  "scripts": {
    "seed": "node --import=vite-plus/register ./scripts/seed.ts",
    "eval": "node --import vite-plus/register --eval \"import('./scripts/seed.ts')\"",
    "test": "node --import vite-plus/register --test",
    "watch": "node --import=vite-plus/register --watch --test ./scripts/seed.ts",
    "tool": "cross-env NODE_OPTIONS=\"--trace-warnings --import vite-plus/register\" mocha",
    "config": "TSX_TSCONFIG_PATH=tsconfig.json node --import vite-plus/register ./scripts/seed.ts",
    "launcher": "NODE_OPTIONS=--import=vite-plus/register concurrently \"node ./scripts/seed.ts\" \"node --test\""
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```
