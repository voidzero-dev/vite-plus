# vpx_script_module_formats

CommonJS and ESM interop, preloads, child processes, and node:test.

## `vpx ./src/both.cts`

A .cts entry that uses export runs as an ES module (no output, exit 0)

```
```

## `vpx ./src/import-cts.ts`

Its named exports import from ESM

```
cts with export, counter=1
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
