# lint_vite_plus_imports_nuxt

## `vp lint --threads=1 src/nuxt.spec.ts src/unit.spec.ts`

bare Vitest imports are exempt in this Nuxt package; the non-config Vite import is preserved (issue #2004); @vitest/browser (not upstream Vitest) still fails

**Exit code:** 1

```

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/test/browser' instead of '@vitest/browser' in Vite+ projects.
   ╭─[src/unit.spec.ts:1:22]
 1 │ import { page } from '@vitest/browser';
   ·                      ─────────────────
 2 │ import { defineConfig } from 'vite';
   ╰────

Found 0 warnings and 1 error.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vp lint --threads=1 --fix src/nuxt.spec.ts src/unit.spec.ts`

fix @vitest/browser without touching upstream Vitest or the preserved Vite import

```
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vpt print-file src/nuxt.spec.ts`

```
import { mockNuxtImport } from '@nuxt/test-utils/runtime';
import { expect, vi } from 'vitest';
import { startVitest } from 'vitest/node';

mockNuxtImport('useExample', () => vi.fn());
void expect;
void startVitest;
```

## `vpt print-file src/unit.spec.ts`

```
import { page } from 'vite-plus/test/browser';
import { defineConfig } from 'vite';
import { expect } from 'vitest';

void page;
void defineConfig;
void expect;
```

## `vp lint --threads=1 src/nuxt.spec.ts src/unit.spec.ts`

confirm the package-level compatible result is clean

```
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
