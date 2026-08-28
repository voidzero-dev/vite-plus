# command_pack_picomatch_hoist_false

## `vp install --ignore-scripts`


## `vp pack src/index.ts`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

node:internal/modules/cjs/loader:1564
  const err = new Error(message);
              ^

Error: Cannot find module 'picomatch'
Require stack:
- <workspace>/node_modules/.pnpm/@voidzero-dev+vite-plus-core@0.1.23_typescript@6.0.3/node_modules/@voidzero-dev/vite-plus-core/dist/tsdown/main-<hash>.js
    at Module._resolveFilename (node:internal/modules/cjs/loader:1564:15)
    at wrapResolveFilename (node:internal/modules/cjs/loader:1118:27)
    at defaultResolveImplForCJSLoading (node:internal/modules/cjs/loader:1142:10)
    at resolveForCJSWithHooks (node:internal/modules/cjs/loader:1169:12)
    at Module._load (node:internal/modules/cjs/loader:1341:5)
    at wrapModuleLoad (node:internal/modules/cjs/loader:261:19)
    at Module.require (node:internal/modules/cjs/loader:1674:12)
    at require (node:internal/modules/helpers:157:16)
    at ModuleJob.run (node:internal/modules/esm/module_job:561:25) {
  code: 'MODULE_NOT_FOUND',
  requireStack: [
    '<workspace>/node_modules/.pnpm/@voidzero-dev+vite-plus-core@0.1.23_typescript@6.0.3/node_modules/@voidzero-dev/vite-plus-core/dist/tsdown/main-<hash>.js'
  ]
}

Node.js <version>
```
