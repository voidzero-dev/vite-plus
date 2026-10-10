# vpx_script_module_formats

CommonJS and ESM interop, preloads, child processes, and node:test.

## `vpx ./src/both.cts`

A .cts that uses export is CommonJS with ES module syntax; it fails with a clear error

**Exit code:** 1

```

node:internal/modules/run_main:123
    triggerUncaughtException(
    ^
Error: Failed to transform <workspace>/src/both.cts: a `.cts` file is CommonJS, but this one uses ES module syntax (`import`, `export` or `import.meta`), which oxc-node does not compile to CommonJS. Rename it to `.mts` to run it as an ES module, or write it as CommonJS with `import x = require("...")` and `export =`.
    at async nextLoad (node:internal/modules/esm/hooks:748:22)
    at async Hooks.load (node:internal/modules/esm/hooks:385:20)
    at async handleMessage (node:internal/modules/esm/worker:199:18) {
  code: 'GenericFailure'
}

Node.js <version>
```

## `vpx ./src/import-cts.ts`

Importing it reports the same error

**Exit code:** 1

```

node:internal/modules/run_main:123
    triggerUncaughtException(
    ^
Error: Failed to transform <workspace>/src/both.cts: a `.cts` file is CommonJS, but this one uses ES module syntax (`import`, `export` or `import.meta`), which oxc-node does not compile to CommonJS. Rename it to `.mts` to run it as an ES module, or write it as CommonJS with `import x = require("...")` and `export =`.
    at async nextLoad (node:internal/modules/esm/hooks:748:22)
    at async Hooks.load (node:internal/modules/esm/hooks:385:20)
    at async handleMessage (node:internal/modules/esm/worker:199:18) {
  code: 'GenericFailure'
}

Node.js <version>
```

## `vpx --require ./src/setup.cts ./src/print-setup.ts`

A TypeScript --require preload runs after the loader

```
required preload ran
```

## `vpx ./src/fork.ts`

fork() and Worker inherit the loader

```
fork: enum from child.ts
worker: enum from worker.ts
```

## `vpx --test --test-reporter=dot ./tests/enum.test.ts`

node:test runs .ts test files

```
.
```
