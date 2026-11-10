# migration_workspace_vite_template

Reproduce #3406: an existing Vite+ workspace converts an independently reusable Vite/Vitest template. This records the current behavior, not the desired exclusion behavior.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

ordinary upgrade without --full also migrates the template

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 2 files had imports rewritten
• Package manager settings configured
```

## `vpt print-file examples/starter/vite.config.ts`

the template's Vite import is converted to vite-plus

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  test: {
    // Vitest v4 compatibility: preserve mock call history.
    // Remove after tests no longer rely on calls from setup or earlier tests.
    // https://viteplus.dev/guide/vitest-v5#remove-unneeded-compatibility-settings
    // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
    clearMocks: false
  }
});
```

## `vpt print-file examples/starter/src/example.test.ts`

the template's Vitest import is converted to vite-plus/test

```
import { expect, test } from 'vite-plus/test';

test('starter', () => {
  expect(1 + 1).toBe(2);
});
```

## `vpt print-file examples/starter/package.json`

record dependency and script changes that also affect standalone use

```
{
  "name": "vite-starter",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build",
    "test": "vp test run"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vitest": "catalog:"
  }
}
```

## `vpt print-file examples/starter/tsconfig.json`

record related type-reference changes

```
{
  "compilerOptions": {
    "types": ["vite/client", "vitest/globals"]
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

the template remains a workspace member and shares toolchain overrides

```
packages:
  - packages/*
  - examples/*
catalog:
  vite-plus: <version>
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
overrides:
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

## `vpt print-file packages/app/package.json`

control: the application continues to use Vite+

```
{
  "name": "app",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vp build"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file packages/app/vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({});
```
