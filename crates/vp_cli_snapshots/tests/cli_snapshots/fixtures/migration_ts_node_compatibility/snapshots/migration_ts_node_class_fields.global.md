# migration_ts_node_class_fields

The local registry fetches pinned ts-node 10.9.2 and TypeScript 5.9.3 to compare real class-field semantics. An existing tsconfig omits target and useDefineForClassFields.

## `vp install --ignore-scripts`


## `vp run cli`

```
VITE+ - The Unified Toolchain for the Web

$ ts-node --transpileOnly fields.ts ⊘ cache disabled
base
```

## `vp run register`

```
VITE+ - The Unified Toolchain for the Web

$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only fields.ts ⊘ cache disabled
base
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• Package manager settings configured
! Warnings:
  - Kept ts-node in package.json scripts "cli": the ts-node config defaults to assignment class fields, while the Vite+ loader defaults to define semantics; set compilerOptions.useDefineForClassFields explicitly before migrating.
  - Kept ts-node in package.json scripts "register": the ts-node config defaults to assignment class fields, while the Vite+ loader defaults to define semantics; set compilerOptions.useDefineForClassFields explicitly before migrating.
  - Kept the ts-node devDependency in package.json: `scripts` in package.json still uses ts-node.
```

## `vpt print-file package.json tsconfig.json`

Both commands and ts-node stay because the implicit class-field defaults differ.

```
{
  "name": "migration-ts-node-compatibility",
  "private": true,
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "cli": "ts-node --transpileOnly fields.ts",
    "register": "TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only fields.ts"
  },
  "devDependencies": {
    "@types/node": "22.18.6",
    "ts-node": "10.9.2",
    "typescript": "5.9.3",
    "vite-plus": "catalog:"
  }
}
{
  "compilerOptions": {
    "module": "CommonJS"
  }
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vp run cli`

```
VITE+ - The Unified Toolchain for the Web

$ ts-node --transpileOnly fields.ts ⊘ cache disabled
base
```

## `vp run register`

```
VITE+ - The Unified Toolchain for the Web

$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only fields.ts ⊘ cache disabled
base
```

## `vpt json-edit tsconfig.json compilerOptions.useDefineForClassFields false`

Make assignment semantics explicit before retrying migration.

```
```

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

● ts-node scripts detected. Auto-migrating supported commands to Vite+...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite   → <version>
• ts-node scripts migrated to Vite+
```

## `vpt print-file package.json tsconfig.json`

```
{
  "name": "migration-ts-node-compatibility",
  "private": true,
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "cli": "vpx --tsconfig=tsconfig.json fields.ts",
    "register": "TS_NODE_PROJECT=tsconfig.json node --import vite-plus/register fields.ts"
  },
  "devDependencies": {
    "@types/node": "22.18.6",
    "typescript": "5.9.3",
    "vite-plus": "catalog:"
  }
}
{
  "compilerOptions": {
    "module": "CommonJS",
    "useDefineForClassFields": false
  }
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vpt stat-file node_modules/ts-node`

```
node_modules/ts-node: missing
```

## `vp run cli`

The migrated loader preserves the base-class value with the explicit setting.

```
VITE+ - The Unified Toolchain for the Web

$ vpx --tsconfig=tsconfig.json fields.ts ⊘ cache disabled
base
```

## `vp run register`

```
VITE+ - The Unified Toolchain for the Web

$ TS_NODE_PROJECT=tsconfig.json node --import vite-plus/register fields.ts ⊘ cache disabled
base
```
