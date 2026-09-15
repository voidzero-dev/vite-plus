# benchmark_preflight

## `vpt write-file example.bench.ts 'import { bench } from '\''vitest'\'';
bench('\''old benchmark'\'', () => 20 + 22, { time: 10 });
'`


## `vp migrate --no-interactive`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item (1 block dependency updates)

example.bench.ts
  1:10 BLOCK [benchmark-api] Migrate these bench references manually: automatic migration requires direct calls with literal names, inline zero-argument callbacks, and no benchmark options. Review wrappers, comparison groups, and escaped references.
    Docs: https://vitest.dev/guide/migration/#benchmarking-api-rewrite
Resolve the blocking Vitest v5 findings, then re-run `vp migrate`. No project files were changed.
```

## `vpt print-file package.json`

```
{
  "name": "migration-vitest-v5",
  "private": true,
  "type": "module",
  "packageManager": "pnpm@11.24.0",
  "scripts": {
    "test": "vitest list"
  },
  "devDependencies": {
    "vite": "^8.0.0",
    "vitest": "<version>"
  }
}
```

## `vpt print-file example.bench.ts`

```
import { bench } from 'vitest';
bench('old benchmark', () => 20 + 22, { time: 10 });
```
