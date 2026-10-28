# vpx_script_module_formats

CommonJS and ESM interop, preloads, child processes, and node:test.

## `vpx ./src/both.cts`

A .cts that uses export is CommonJS with ES module syntax; it fails with a clear error

**Exit code:** 1

```
<repo>/packages/cli/dist/script-preload.cjs:454
        const result = oxcNode.load(url, context, nextLoad, HELPER_MODULE_NAME);
                               ^

Error: Failed to transform <workspace>/src/both.cts: a `.cts` file is CommonJS, but this one uses ES module syntax (`import`, `export` or `import.meta`), which oxc-node does not compile to CommonJS. Rename it to `.mts` to run it as an ES module, or write it as CommonJS with `import x = require("...")` and `export =`.
    at load (<repo>/packages/cli/dist/script-preload.cjs:454:25)
    at nextStep (node:internal/modules/customization_hooks:189:26)
    at loadWithHooks (node:internal/modules/customization_hooks:374:18)
    at #loadSync (node:internal/modules/esm/loader:865:14)
    at ModuleLoader.load (node:internal/modules/esm/loader:824:28)
    at ModuleLoader.loadAndTranslate (node:internal/modules/esm/loader:609:31)
    at #createModuleJob (node:internal/modules/esm/loader:640:36)
    at #getJobFromResolveResult (node:internal/modules/esm/loader:353:34)
    at ModuleLoader.getModuleJobForImport (node:internal/modules/esm/loader:321:41)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:680:25) {
  code: 'GenericFailure'
}

Node.js <version>
```

## `vpx ./src/import-cts.ts`

Importing it reports the same error

**Exit code:** 1

```
<repo>/packages/cli/dist/script-preload.cjs:454
        const result = oxcNode.load(url, context, nextLoad, HELPER_MODULE_NAME);
                               ^

Error: Failed to transform <workspace>/src/both.cts: a `.cts` file is CommonJS, but this one uses ES module syntax (`import`, `export` or `import.meta`), which oxc-node does not compile to CommonJS. Rename it to `.mts` to run it as an ES module, or write it as CommonJS with `import x = require("...")` and `export =`.
    at load (<repo>/packages/cli/dist/script-preload.cjs:454:25)
    at nextStep (node:internal/modules/customization_hooks:189:26)
    at loadWithHooks (node:internal/modules/customization_hooks:374:18)
    at #loadSync (node:internal/modules/esm/loader:865:14)
    at ModuleLoader.load (node:internal/modules/esm/loader:824:28)
    at ModuleLoader.loadAndTranslate (node:internal/modules/esm/loader:609:31)
    at #createModuleJob (node:internal/modules/esm/loader:640:36)
    at #getJobFromResolveResult (node:internal/modules/esm/loader:353:34)
    at ModuleLoader.getModuleJobForImport (node:internal/modules/esm/loader:321:41) {
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

## `vpx --test --test-reporter=dot ./tests/**/*.test.ts`

A test glob reaches node:test as a pattern, not a script path

```
.
```

## `vpx ./src/data-url.ts`

data: URL imports keep Node.js' handling

```
data: URL import: 7
```

## `vpx ./src/require-tla.cts`

require() of a module with top-level await reports Node.js' error code

```
require() of top-level await: ERR_REQUIRE_ASYNC_MODULE
```
