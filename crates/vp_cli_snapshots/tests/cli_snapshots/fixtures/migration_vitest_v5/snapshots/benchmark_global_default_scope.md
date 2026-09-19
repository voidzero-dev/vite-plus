# benchmark_global_default_scope

Migrate globals in default benchmark paths independently of test patterns, preserve an unrelated runner's file, and execute the parenthesized workload.

## `vpt write-file vite.config.ts 'export default { test: { globals: true } };
'`


## `vpt json-edit package.json scripts.bench 'vitest bench --run'`


## `vpt write-file example.bench.js 'bench('\''work'\'', (() => { for (let i = 0; i < 100; i++) JSON.parse('\''{"value":42}'\''); }));
'`


## `vpt write-file unrelated.js 'bench('\''other runner'\'', () => { expect(() => {}).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file example.bench.js unrelated.js`

```
globalThis.test('work', (async ({ bench }) => { await bench('work', () => { for (let i = 0; i < 100; i++) JSON.parse('{"value":42}'); }).run(); }));
bench('other runner', () => { expect(() => {}).toThrow(''); });
```

## `vp test bench --run --reporter=json --outputFile=benchmark.json`


## `node verify-benchmark-regressions.mjs example.bench.js work`

```
Vitest 5 executed 1 migrated benchmarks in example.bench.js
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
