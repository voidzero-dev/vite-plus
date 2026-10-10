# vpx_script_node24

From Node.js 24.11.1 (and 22.22.3) the loader uses in-thread module.registerHooks() instead of a module.register() hooks thread.

## `vpx ./main.ts`

```
registerHooks commonjs .cts true
```

## `vpx ./import-cjs.ts`

An imported CommonJS module can require() other files

```
nested commonjs .cts
```

## `vpx ./import-both.ts`

A .cts with ES module syntax fails with the same error on the in-thread path

```
Failed to transform file://<workspace>/both.cts: a `.cts` file is CommonJS, but this one uses ES module syntax (`import`, `export` or `import.meta`), which oxc-node does not compile to CommonJS. Rename it to `.mts` to run it as an ES module, or write it as CommonJS with `import x = require("...")` and `export =`.
```
