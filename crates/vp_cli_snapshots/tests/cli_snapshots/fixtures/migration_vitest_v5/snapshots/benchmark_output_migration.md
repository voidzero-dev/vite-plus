# benchmark_output_migration

Migrate dynamic names, local workloads, built-in reporters, and config/CLI JSON paths. Run both CLI forms and verify the new report data.

## `vpt write-file vite.config.ts 'export default { test: { include: ['\''unused.test.js'\''], benchmark: { include: ['\''example.bench.ts'\''], reporters: '\''verbose'\'', outputJson: '\''reports/config.json'\'' } } };
'`


## `vpt json-edit package.json scripts.bench 'vitest bench --run --outputJson='\''reports/cli bench.json'\'''`


## `vpt write-file example.bench.ts 'import { bench, describe } from '\''vitest'\'';
describe('\''utilities'\'', () => {
  let calls = 0;
  const name = () => { if (++calls '\!'== 1) throw new Error('\''name evaluated twice'\''); return '\''parse'\''; };
  const json = '\''{"value":42}'\'';
  function workload() { if (JSON.parse(json).value '\!'== 42) throw new Error('\''scope lost'\''); }
  bench(name(), workload);
});
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file example.bench.ts vite.config.ts package.json`

```
import { test, describe } from 'vite-plus/test';
describe('utilities', () => {
  let calls = 0;
  const name = () => { if (++calls !== 1) throw new Error('name evaluated twice'); return 'parse'; };
  const json = '{"value":42}';
  function workload() { if (JSON.parse(json).value !== 42) throw new Error('scope lost'); }
  { const _benchName = (name()); const _benchFn = workload; test(_benchName, async ({ bench }) => { await bench(_benchName, _benchFn).run(); }); }
});
export default {
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  test: {
      // Vitest v4 compatibility: preserve mock call history.
      // Remove after tests no longer rely on calls from setup or earlier tests.
      // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
      // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
      clearMocks: false,
      reporters: ["verbose","json"], outputFile: { json: "reports/config.json" },  include: ['unused.test.js'], benchmark: { include: ['example.bench.ts'],   } }
}
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-vitest-v5",
  "packageManager": "pnpm@11.24.0",
  "private": true,
  "scripts": {
    "bench": "vp test bench --run --reporter=default --reporter=json --outputFile='reports/cli bench.json'",
    "test": "vp test list --no-static-parse"
  },
  "type": "module"
}
```

## `vp test bench --run`


## `vp run bench`


## `node verify-benchmark-output.mjs reports/config.json 'reports/cli bench.json'`

```
reports/config.json: migrated benchmark passed and JSON includes its result
reports/cli bench.json: migrated benchmark passed and JSON includes its result
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
