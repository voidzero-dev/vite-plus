# benchmark_global_custom_scope

Use benchmark include/exclude independently of test exclusions. Preserve excluded and out-of-scope files, including the overridden default benchmark path.

## `vpt write-file vite.config.ts 'export default { test: { globals: true, exclude: ['\''perf/**'\''], benchmark: { include: ['\''perf/*.js'\''], exclude: ['\''perf/excluded.js'\''] } } };
'`


## `vpt json-edit package.json scripts.bench 'vitest bench --run'`


## `vpt write-file perf/work.js 'bench('\''work'\'', () => { for (let i = 0; i < 100; i++) JSON.parse('\''{"value":42}'\''); });
'`


## `vpt write-file perf/excluded.js 'bench('\''excluded'\'', () => { expect(() => {}).toThrow('\'''\''); });
'`


## `vpt write-file example.bench.js 'bench('\''other runner'\'', () => { expect(() => {}).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file perf/work.js perf/excluded.js example.bench.js`

```
globalThis.test('work', async ({ bench }) => { await bench('work', () => { for (let i = 0; i < 100; i++) JSON.parse('{"value":42}'); }).run(); });
bench('excluded', () => { expect(() => {}).toThrow(''); });
bench('other runner', () => { expect(() => {}).toThrow(''); });
```

## `vp test bench --run --reporter=json --outputFile=benchmark.json`


## `node verify-benchmark-regressions.mjs perf/work.js work`

```
Vitest 5 executed 1 migrated benchmarks in perf/work.js
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
