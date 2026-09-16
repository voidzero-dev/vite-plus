# benchmark_in_source_preflight

Block unsupported in-source benchmark options before dependency changes and retain the guarded source.

## `vpt write-file vite.config.ts 'export default { test: { benchmark: { include: [], includeSource: ['\''perf/*.js'\''] } } };
'`


## `vpt json-edit package.json scripts.bench 'vitest bench --run'`


## `vpt write-file perf/work.js 'if (import.meta.vitest) {
  const { bench } = import.meta.vitest;
  bench('\''work'\'', () => 42, { time: 1 });
}
'`


## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item (1 block dependency updates)

perf/work.js
  2:11 BLOCK [benchmark-api] Migrate this import.meta.vitest bench reference manually: automatic migration requires direct calls with locally resolved zero-argument callbacks and no benchmark options or escaped references. Keep the import.meta.vitest guard.
    Docs: https://vitest.dev/guide/migration/#benchmarking-api-rewrite
Resolve the blocking Vitest v5 findings, then re-run `vp migrate`. No project files were changed.
```

## `vpt print-file package.json perf/work.js`

```
{
  "devDependencies": {
    "vite": "^8.0.0",
    "vitest": "<version>"
  },
  "name": "migration-vitest-v5",
  "packageManager": "pnpm@11.24.0",
  "private": true,
  "scripts": {
    "bench": "vitest bench --run",
    "test": "vitest list"
  },
  "type": "module"
}
if (import.meta.vitest) {
  const { bench } = import.meta.vitest;
  bench('work', () => 42, { time: 1 });
}
```
