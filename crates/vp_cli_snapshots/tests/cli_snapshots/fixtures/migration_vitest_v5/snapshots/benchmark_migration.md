# benchmark_migration

Migrate direct benchmarks, preserve suite closures and modifiers, and execute the migrated workloads with Vitest 5. Keep the supported bench command.

## `vpt write-file vite.config.ts 'export default { test: { include: ['\''unused.test.js'\''], benchmark: { include: ['\''example.bench.ts'\''] } } };
'`


## `vpt json-edit package.json scripts.bench 'vitest bench --run'`


## `vpt write-file example.bench.ts 'import { bench, describe } from '\''vitest'\'';
describe('\''utilities'\'', () => {
  const json = '\''{"value":42}'\'';
  bench('\''parse'\'', () => { if (JSON.parse(json).value '\!'== 42) throw new Error('\''scope lost'\''); });
  bench('\''async'\'', async () => { if (await Promise.resolve(json) '\!'== json) throw new Error('\''scope lost'\''); });
  bench.skip('\''skipped'\'', () => { throw new Error('\''must not run'\''); });
  bench.todo('\''later'\'');
});
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file example.bench.ts`

```
import { test, describe } from 'vite-plus/test';
describe('utilities', () => {
  const json = '{"value":42}';
  test('parse', async ({ bench }) => { await bench('parse', () => { if (JSON.parse(json).value !== 42) throw new Error('scope lost'); }).run(); });
  test('async', async ({ bench }) => { await bench('async', async () => { if (await Promise.resolve(json) !== json) throw new Error('scope lost'); }).run(); });
  test.skip('skipped', async ({ bench }) => { await bench('skipped', () => { throw new Error('must not run'); }).run(); });
  test.todo('later');
});
```

## `vpt print-file package.json`

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-vitest-v5",
  "packageManager": "pnpm@11.24.0",
  "private": true,
  "scripts": {
    "bench": "vp test bench --run",
    "test": "vp test list --no-static-parse"
  },
  "type": "module"
}
```

## `node verify-benchmarks.mjs`

```
Vitest 5 executed both migrated benchmarks and preserved skip/todo modifiers
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
