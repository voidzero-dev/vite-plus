# benchmark_parenthesized_workloads

Migrate and run parenthesized arrows, function expressions, and local callbacks without losing parentheses, comments, or trailing commas.

## `vpt write-file vite.config.ts 'export default { test: {} };
'`


## `vpt json-edit package.json scripts.bench 'vitest bench --run'`


## `vpt write-file example.bench.js 'import { bench } from '\''vitest'\'';
const workload = () => { for (let i = 0; i < 100; i++) JSON.parse('\''{"value":42}'\''); };
bench('\''arrow'\'', (() => workload()));
bench('\''nested'\'', ((() => workload())));
bench('\''function'\'', (function () { workload(); }));
bench('\''local'\'', (workload));
bench('\''comments'\'', (/* before */ (() => workload()) /* after */), /* trailing */);
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file example.bench.js`

```
import { test } from 'vite-plus/test';
const workload = () => { for (let i = 0; i < 100; i++) JSON.parse('{"value":42}'); };
test('arrow', (async ({ bench }) => { await bench('arrow', () => workload()).run(); }));
test('nested', ((async ({ bench }) => { await bench('nested', () => workload()).run(); })));
test('function', (async ({ bench }) => { await bench('function', function () { workload(); }).run(); }));
{ const _benchFn = workload; test('local', (async ({ bench }) => { await bench('local', _benchFn).run(); })); }
test('comments', (/* before */ (async ({ bench }) => { await bench('comments', () => workload()).run(); }) /* after */), /* trailing */);
```

## `vp test bench --run --reporter=json --outputFile=benchmark.json`


## `node verify-benchmark-regressions.mjs example.bench.js arrow nested function local comments`

```
Vitest 5 executed 5 migrated benchmarks in example.bench.js
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
