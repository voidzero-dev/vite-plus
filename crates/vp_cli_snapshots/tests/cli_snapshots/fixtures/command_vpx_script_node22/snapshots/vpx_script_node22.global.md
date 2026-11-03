# vpx_script_node22

Node.js 22.22.3+ uses the in-thread module.registerHooks() too, without nodejs/node#62920, and lowers `using` to a runtime helper.

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
Failed to transform <workspace>/both.cts: a `.cts` file is CommonJS, but this one uses ES module syntax (`import`, `export` or `import.meta`), which oxc-node does not compile to CommonJS. Rename it to `.mts` to run it as an ES module, or write it as CommonJS with `import x = require("...")` and `export =`.
```

## `cd cjs && vpx ./deco.ts`

CommonJS output require()s runtime helpers that the project cannot resolve itself

```
decorated run
commonjs object, count 1
```
