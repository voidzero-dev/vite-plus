# migration_upgrade_nuxt_test_utils

## `vp migrate --no-interactive`

preserve upstream Vitest throughout packages that declare @nuxt/test-utils

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
    vitest     4.0.2  → <version>
• 1 file had imports rewritten
• Kept upstream `vitest` imports in 2 files for @nuxt/test-utils compatibility
• Package manager settings configured
```

## `vpt print-file package.json`

direct Vitest and its shared pin remain for the package-level exception

```
{
  "name": "migration-upgrade-nuxt-test-utils",
  "devDependencies": {
    "@nuxt/test-utils": "file:.fixture/nuxt-test-utils",
    "vite-plus": "<version>",
    "vitest": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "npm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file nuxt.spec.ts`

unscoped Vitest stays while the scoped browser package migrates

```
import { mockNuxtImport } from '@nuxt/test-utils/runtime';
import { page } from 'vite-plus/test/browser/context';
import { vi } from 'vitest';
import { defineConfig } from 'vitest/config';

mockNuxtImport('useExample', () => vi.fn());
void page;
void defineConfig;
```

## `vpt print-file unit.spec.ts`

an unrelated test file in the same package also keeps upstream Vitest

```
import { expect } from 'vitest';

expect(true).toBe(true);
```

## `vp migrate --no-interactive`

the package-level compatibility result is idempotent

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
