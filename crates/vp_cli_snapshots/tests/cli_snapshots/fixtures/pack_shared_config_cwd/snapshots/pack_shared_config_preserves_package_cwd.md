# pack_shared_config_preserves_package_cwd

## `vp pack --logLevel silent`

Load the parent config through traverseUp while plugin hooks and pack paths stay at the package cwd, overriding the file's root.

```
apply root: <workspace>/packages/lib
apply cwd: <workspace>/packages/lib
config root: <workspace>/packages/lib
config cwd: <workspace>/packages/lib
configResolved root: <workspace>/packages/lib
configResolved cwd: <workspace>/packages/lib
configResolved configFile: <workspace>/vite.config.mjs
ℹ Build start
```

## `vpt print-file dist/index.mjs`

```
// src/index.js
const source = "package source";
export { source };
```

## `vpt stat-file ../../dist ../../configured-root/dist`

```
../../dist: missing
../../configured-root/dist: missing
```
