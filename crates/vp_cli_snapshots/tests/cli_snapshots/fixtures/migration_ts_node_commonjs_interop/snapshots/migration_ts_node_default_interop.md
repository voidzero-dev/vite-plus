# migration_ts_node_default_interop

esModuleInterop=true still unwraps __esModule.default in ts-node. Node's native default import returns the whole exports object, so migration must retain this command too.

## `vpt json-edit package.json scripts '{"cli":"ts-node -T default.ts","register":"TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only default.ts"}'`


## `vpt json-edit tsconfig.json compilerOptions.esModuleInterop true`


## `vpt write-file default.cjs 'exports.__esModule = true; exports.default = 42;
'`


## `vp install --ignore-scripts`


## `vp run cli`

```
$ ts-node -T default.ts ⊘ cache disabled
42
```

## `vp run register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only default.ts ⊘ cache disabled
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
  - Kept ts-node in package.json scripts "cli": `default.ts` has runtime bindings from `./default.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "register": `default.ts` has runtime bindings from `./default.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept the ts-node devDependency in package.json: `scripts` in package.json still uses ts-node.
```

## `vpt print-file package.json tsconfig.json default.cjs`

```
{
  "devDependencies": {
    "@types/node": "22.18.6",
    "ts-node": "10.9.2",
    "typescript": "5.9.3",
    "vite-plus": "catalog:"
  },
  "name": "migration-ts-node-commonjs-interop",
  "packageManager": "pnpm@12.11.2",
  "private": true,
  "scripts": {
    "cli": "ts-node -T default.ts",
    "register": "TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only default.ts"
  }
}
{
  "compilerOptions": {
    "esModuleInterop": true,
    "module": "CommonJS",
    "target": "ES2022"
  }
}
exports.__esModule = true; exports.default = 42;
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vpt stat-file node_modules/ts-node`

```
node_modules/ts-node: symlink
```

## `vp run cli`

```
$ ts-node -T default.ts ⊘ cache disabled
42
```

## `vp run register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only default.ts ⊘ cache disabled
42
```
