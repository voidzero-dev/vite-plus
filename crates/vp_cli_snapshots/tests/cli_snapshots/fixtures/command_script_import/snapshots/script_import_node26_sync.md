# script_import_node26_sync

The complete preload contract runs on Node.js 26 synchronous hooks.

## `node verify.mjs`

```
public package export: both flag forms, resolved URL, NODE_OPTIONS, ESM/CJS
subsequent TypeScript preload, argv, execArgv, PID, runtime, and exit status
eval/stdin imports and requires, dynamic registration, node:test and discovery
fork, ESM/CJS workers, spawned Node.js, and inherited custom tsconfig
worker-specific configs, explicit worker preload, cluster, and internal loader thread
repeated flags, repeated dynamic imports, and NODE_OPTIONS/argv overlap
config paths with spaces, explicit/compat/upstream precedence, unchanged environment
source maps identify TypeScript lines in application and dependency errors
```

## `vpx --import=vite-plus/register ./src/dedup.mjs`

```
registration stays single across repeated and mixed preloads
```
