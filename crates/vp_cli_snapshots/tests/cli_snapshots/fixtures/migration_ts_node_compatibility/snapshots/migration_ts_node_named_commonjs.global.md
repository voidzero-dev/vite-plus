# migration_ts_node_named_commonjs

Real ts-node compiles the named import to require() property access. The CommonJS export object is built dynamically so Node cannot infer the named export statically.

## `vpt json-edit tsconfig.json compilerOptions.target ES2022`


## `vpt json-edit package.json scripts.cli 'ts-node -T named.ts'`


## `vpt json-edit package.json scripts.register 'TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only named.ts'`


## `vp install --ignore-scripts`


## `vp run cli`

```
VITE+ - The Unified Toolchain for the Web

$ ts-node -T named.ts ⊘ cache disabled
42
```

## `vp run register`

```
VITE+ - The Unified Toolchain for the Web

$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only named.ts ⊘ cache disabled
42
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
  - Kept ts-node in package.json scripts "cli": `named.ts` uses named imports from CommonJS, which `vpx` leaves to Node.js instead of compiling to require(); migrate the interop manually.
  - Kept ts-node in package.json scripts "register": `named.ts` uses named imports from CommonJS, which `vpx` leaves to Node.js instead of compiling to require(); migrate the interop manually.
  - Kept the ts-node devDependency in package.json: `scripts` in package.json still uses ts-node.
```

## `vpt print-file package.json named.ts dep.cjs`

The named import keeps both commands and the ts-node dependency.

```
{
  "devDependencies": {
    "@types/node": "22.18.6",
    "ts-node": "10.9.2",
    "typescript": "5.9.3",
    "vite-plus": "catalog:"
  },
  "name": "migration-ts-node-compatibility",
  "packageManager": "pnpm@12.11.2",
  "private": true,
  "scripts": {
    "cli": "ts-node -T named.ts",
    "register": "TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only named.ts"
  }
}
import { value } from './dep.cjs';
console.log(value);
module.exports = Object.assign({}, { value: 42 });
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vpt stat-file node_modules/ts-node`

```
node_modules/ts-node: symlink
```

## `vp run cli`

The preserved CommonJS compilation still prints 42 after a fresh install.

```
VITE+ - The Unified Toolchain for the Web

$ ts-node -T named.ts ⊘ cache disabled
42
```

## `vp run register`

```
VITE+ - The Unified Toolchain for the Web

$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only named.ts ⊘ cache disabled
42
```
