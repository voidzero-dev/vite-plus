# migration_ts_node_commonjs_interop

The local registry fetches pinned ts-node 10.9.2 and TypeScript 5.9.3 to compare real CommonJS bindings. Re-exports, namespace imports, .js targets, and default imports must retain their values after migration and a fresh install. Checked ES module bindings still migrate.

## `vp install --ignore-scripts`


## `vp run reexport`

```
$ ts-node -T reexport.ts ⊘ cache disabled
loaded
```

## `vp run reexport-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only reexport.ts ⊘ cache disabled
loaded
```

## `vp run namespace`

```
$ ts-node -T namespace.ts ⊘ cache disabled
42
```

## `vp run namespace-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only namespace.ts ⊘ cache disabled
42
```

## `vp run js`

```
$ ts-node -T named-js.ts ⊘ cache disabled
42
```

## `vp run js-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only named-js.ts ⊘ cache disabled
42
```

## `vp run default`

```
$ ts-node -T default.ts ⊘ cache disabled
42
```

## `vp run default-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only default.ts ⊘ cache disabled
42
```

## `vp run safe`

```
$ ts-node -T safe.ts ⊘ cache disabled
42 42 42
```

## `vp run safe-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only safe.ts ⊘ cache disabled
42 42 42
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
• ts-node scripts migrated to Vite+
• Package manager settings configured
! Warnings:
  - Kept ts-node in package.json scripts "reexport": `reexport.ts` has runtime bindings from `./dep.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "reexport-register": `reexport.ts` has runtime bindings from `./dep.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "namespace": `namespace.ts` has runtime bindings from `./dep.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "namespace-register": `namespace.ts` has runtime bindings from `./dep.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "js": `named-js.ts` has runtime bindings from `./dep.js` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "js-register": `named-js.ts` has runtime bindings from `./dep.js` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "default": `default.ts` has runtime bindings from `./default.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept ts-node in package.json scripts "default-register": `default.ts` has runtime bindings from `./default.cjs` whose CommonJS interop or import/require resolution may change with `vpx`; migrate the bindings manually.
  - Kept the ts-node devDependency in package.json: `scripts` in package.json still uses ts-node.
```

## `vpt print-file package.json tsconfig.json`

CommonJS commands and ts-node stay; the checked ES module commands migrate.

```
{
  "name": "migration-ts-node-commonjs-interop",
  "private": true,
  "packageManager": "pnpm@12.11.2",
  "scripts": {
    "reexport": "ts-node -T reexport.ts",
    "reexport-register": "TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only reexport.ts",
    "namespace": "ts-node -T namespace.ts",
    "namespace-register": "TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only namespace.ts",
    "js": "ts-node -T named-js.ts",
    "js-register": "TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only named-js.ts",
    "default": "ts-node -T default.ts",
    "default-register": "TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only default.ts",
    "safe": "vpx --tsconfig=tsconfig.json safe.ts",
    "safe-register": "TS_NODE_PROJECT=tsconfig.json node --import vite-plus/register safe.ts"
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
    "module": "CommonJS",
    "target": "ES2022",
    "esModuleInterop": false
  }
}
```

## `vpt rm -rf node_modules`


## `vp install --ignore-scripts`


## `vpt stat-file node_modules/ts-node`

```
node_modules/ts-node: symlink
```

## `vp run reexport`

```
$ ts-node -T reexport.ts ⊘ cache disabled
loaded
```

## `vp run reexport-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only reexport.ts ⊘ cache disabled
loaded
```

## `vp run namespace`

```
$ ts-node -T namespace.ts ⊘ cache disabled
42
```

## `vp run namespace-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only namespace.ts ⊘ cache disabled
42
```

## `vp run js`

```
$ ts-node -T named-js.ts ⊘ cache disabled
42
```

## `vp run js-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only named-js.ts ⊘ cache disabled
42
```

## `vp run default`

```
$ ts-node -T default.ts ⊘ cache disabled
42
```

## `vp run default-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --require ts-node/register/transpile-only default.ts ⊘ cache disabled
42
```

## `vp run safe`

```
$ vpx --tsconfig=tsconfig.json safe.ts ⊘ cache disabled
42 42 42
```

## `vp run safe-register`

```
$ TS_NODE_PROJECT=tsconfig.json node --import vite-plus/register safe.ts ⊘ cache disabled
42 42 42
```
