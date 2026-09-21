# benchmark_in_source

Migrate destructured and direct import.meta.vitest benchmarks with default globals. Execute the migrated benchmarks and import the production module without Vitest.

## `vpt write-file vite.config.ts 'export default { test: { benchmark: { include: [], includeSource: ['\''perf/*.js'\''] } } };
'`


## `vpt json-edit package.json scripts.bench 'vitest bench --run'`


## `vpt write-file perf/work.js 'export function work() { for (let i = 0; i < 100; i++) JSON.parse('\''{"value":42}'\''); return 42; }
if (import.meta.vitest) {
  const { bench } = import.meta.vitest;
  bench('\''destructured'\'', () => work());
  import.meta.vitest.bench('\''member'\'', (() => work()));
}
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file perf/work.js`

```
export function work() { for (let i = 0; i < 100; i++) JSON.parse('{"value":42}'); return 42; }
if (import.meta.vitest) {
  const { test } = import.meta.vitest;
  test('destructured', async ({ bench }) => { await bench('destructured', () => work()).run(); });
  import.meta.vitest.test('member', (async ({ bench }) => { await bench('member', () => work()).run(); }));
}
```

## `vp test bench --run --reporter=json --outputFile=benchmark.json`


## `node verify-benchmark-regressions.mjs perf/work.js destructured member`

```
Vitest 5 executed 2 migrated benchmarks in perf/work.js
```

## `node --input-type=module -e 'import assert from '\''node:assert/strict'\''; import { work } from '\''./perf/work.js'\''; assert.equal(work(), 42); console.log('\''Production import works without Vitest'\'');'`

```
Production import works without Vitest
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
