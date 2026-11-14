# migration_import_rule_full

Full migration and check autofix share the import rule; other lint rules remain active.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor --full`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 3 files had imports rewritten
• Package manager settings configured
```

## `vpt print-file templates/starter/vite.config.ts`

```
import { defineConfig } from 'vite';
export default defineConfig({test: {
    // Vitest v4 compatibility: preserve mock call history.
    // Remove after tests no longer rely on calls from setup or earlier tests.
    // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
    // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
    clearMocks: false,
    poolOptions: {threads: {singleThread: true}}}});
```

## `vpt print-file templates/starter/unit.test.ts`

```
import { expect, test } from 'vitest';
debugger;
test('template', () => expect(1).toBe(1));
```

## `vpt print-file templates/managed/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('included', () => expect(1).toBe(1));
```

## `vpt print-file examples/standalone/unit.test.ts`

```
import { expect, test } from 'vitest';
test('outside-workspace', () => expect(1).toBe(1));
```

## `vpt print-file packages/app/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('application', () => expect(1).toBe(1));
```

## `vpt print-file templates/starter/package.json`

```
{
  "name": "starter",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vp build",
    "test": "vp test run"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vitest": "catalog:legacy",
    "@vitest/coverage-v8": "catalog:legacy"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

```
packages:
  - packages/*
  - templates/*
catalog:
  vite-plus: <version>
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  '@vitest/coverage-v8': <version>
catalogs:
  legacy:
    vitest: <version>
    '@vitest/coverage-v8': <version>
overrides:
  unrelated-package: 1.0.0
  vite@*: 'catalog:'
  vitest@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

## `vpt write-file packages/app/unit.test.ts 'import { test } from '\''vitest'\'';
test('\''autofix'\'', () => {});
'`

```
```

## `vp check --no-fmt --fix templates/starter/unit.test.ts packages/app/unit.test.ts`

**Exit code:** 1

```
error: Lint issues found
× eslint(no-debugger): `debugger` statement is not allowed
   ╭─[templates/starter/unit.test.ts:2:1]
 1 │ import { expect, test } from 'vitest';
 2 │ debugger;
   · ─────────
 3 │ test('template', () => expect(1).toBe(1));
   ╰────
  help: Remove the debugger statement

Found 1 error and 0 warnings in 2 files (<duration>, <n> threads)
```

## `vpt print-file templates/starter/unit.test.ts`

```
import { expect, test } from 'vitest';
debugger;
test('template', () => expect(1).toBe(1));
```

## `vpt print-file packages/app/unit.test.ts`

```
import { test } from 'vite-plus/test';
test('autofix', () => {});
```
