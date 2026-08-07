# lint_vite_plus_imports

## `vp lint src/index.ts`

should fail before fix (index.ts)

**Exit code:** 1

```

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus' instead of 'vitest/config' in Vite+ projects.
   ╭─[src/index.ts:3:30]
 2 │
 3 │ const configPromise = import('vitest/config');
   ·                              ───────────────
 4 │
   ╰────

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/test' instead of 'vitest' in Vite+ projects.
   ╭─[src/index.ts:5:24]
 4 │
 5 │ export { expect } from 'vitest';
   ·                        ────────
 6 │
   ╰────

Found 0 warnings and 2 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint src/types.ts`

should fail before fix (types.ts)

**Exit code:** 1

```

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/test' instead of 'vitest' in Vite+ projects.
   ╭─[src/types.ts:1:30]
 1 │ type TestFn = (typeof import('vitest'))['test'];
   ·                              ────────
 2 │ type BrowserContext = typeof import('@vitest/browser/context');
   ╰────

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/test/browser/context' instead of '@vitest/browser/context' in Vite+ projects.
   ╭─[src/types.ts:2:37]
 1 │ type TestFn = (typeof import('vitest'))['test'];
 2 │ type BrowserContext = typeof import('@vitest/browser/context');
   ·                                     ─────────────────────────
 3 │ type BrowserClient = typeof import('@vitest/browser/client');
   ╰────

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/test/client' instead of '@vitest/browser/client' in Vite+ projects.
   ╭─[src/types.ts:3:36]
 2 │ type BrowserContext = typeof import('@vitest/browser/context');
 3 │ type BrowserClient = typeof import('@vitest/browser/client');
   ·                                    ────────────────────────
 4 │ type PlaywrightProvider = typeof import('@vitest/browser-playwright/provider');
   ╰────

  × vite-plus(prefer-vite-plus-imports): Use 'vite-plus/test/browser/providers/playwright' instead of '@vitest/browser-playwright/provider' in Vite+ projects.
   ╭─[src/types.ts:4:41]
 3 │ type BrowserClient = typeof import('@vitest/browser/client');
 4 │ type PlaywrightProvider = typeof import('@vitest/browser-playwright/provider');
   ·                                         ─────────────────────────────────────
 5 │
   ╰────

Found 0 warnings and 4 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp lint --fix src/index.ts src/types.ts`

rewrite vitest/@vitest imports; vite imports are preserved outside config files (issue #2004)

```
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vpt print-file src/index.ts`

```
import { defineConfig } from 'vite';

const configPromise = import('vite-plus');

export { expect } from 'vite-plus/test';

void defineConfig;
void configPromise;
```

## `vpt print-file src/types.ts`

```
type TestFn = (typeof import('vite-plus/test'))['test'];
type BrowserContext = typeof import('vite-plus/test/browser/context');
type BrowserClient = typeof import('vite-plus/test/client');
type PlaywrightProvider = typeof import('vite-plus/test/browser/providers/playwright');

declare module '@vitest/browser-playwright' {}
declare module '@vitest/browser-playwright/context' {}

import client = require('vite/client');

export type { BrowserClient, BrowserContext, PlaywrightProvider, TestFn };

void client;
```

## `vp lint src/index.ts src/types.ts`

confirm the rewritten files are clean

```
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
