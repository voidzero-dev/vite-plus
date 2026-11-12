# migration_import_rule_fresh

Initial migration respects the same import rule configuration.

## `vpt json-edit package.json devDependencies {}`

```
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied, 3 files had imports rewritten
! Warnings:
  - Removed `"esModuleInterop": false` from templates/starter/tsconfig.json — this option has been deprecated. See https://github.com/oxc-project/tsgolint/issues/351, https://github.com/microsoft/TypeScript/issues/62529
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
    "@vitest/coverage-v8": "catalog:legacy",
    "vite-plus": "catalog:"
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
