# vpx_script_node26

On Node.js 26 the loader uses in-thread module.registerHooks(), without the DEP0205 warning module.register() prints.

## `vpx ./main.ts`

```
registerHooks commonjs .cts true
```

## `vpx ./import-both.ts`

A .cts with ES module syntax fails with the same error on the in-thread path

```
Failed to transform file://<workspace>/both.cts: a `.cts` file is CommonJS, but this one uses ES module syntax (`import`, `export` or `import.meta`), which oxc-node does not compile to CommonJS. Rename it to `.mts` to run it as an ES module, or write it as CommonJS with `import x = require("...")` and `export =`.
```
