# migration_upgrade_nuxt_test_utils_monorepo

## `vp migrate --no-interactive`

preserve upstream Vitest package-wide and localize it to the affected workspace

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items

packages/nuxt/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.

packages/unit/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 1 file had imports rewritten
• Kept upstream `vitest` imports in 2 files for @nuxt/test-utils compatibility
• Package manager settings configured
! Warnings:
  - Vitest v5: 2 review items

packages/nuxt/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.

packages/unit/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file packages/nuxt/package.json`

affected workspace keeps direct Vitest

```
{
  "name": "nuxt-tests",
  "private": true,
  "devDependencies": {
    "@nuxt/test-utils": "file:../../.fixture/nuxt-test-utils",
    "vitest": "catalog:"
  }
}
```

## `vpt print-file packages/unit/package.json`

unrelated workspace drops direct Vitest

```
{
  "name": "unit-tests",
  "private": true,
  "devDependencies": {}
}
```

## `vpt print-file pnpm-workspace.yaml`

shared Vitest pin remains because one workspace needs it

```
packages:
  - packages/*

catalog:
  vite-plus: <version>
  vitest: <version>
  vite: npm:@voidzero-dev/vite-plus-core@<version>

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

## `vpt print-file packages/nuxt/nuxt.spec.ts`

upstream Vitest and its subpath stay

```
import { mockNuxtImport } from '@nuxt/test-utils/runtime';
import { expect, vi } from 'vitest';
import { startVitest } from 'vitest/node';

mockNuxtImport('useExample', () => vi.fn());
void expect;
void startVitest;
```

## `vpt print-file packages/nuxt/unit.spec.ts`

files without Nuxt imports still preserve Vitest in the affected package

```
import { expect } from 'vitest';
import { startVitest } from 'vitest/node';

void expect;
void startVitest;
```

## `vpt print-file packages/unit/unit.spec.ts`

an unrelated workspace still migrates Vitest

```
import { expect } from 'vite-plus/test';

void expect;
```

## `vp migrate --no-interactive`

workspace result is idempotent

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items

packages/nuxt/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.

packages/unit/package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
This project is already using Vite+! Happy coding!
```
