# pnpm_peer_identity_and_test_cache

Regression for #1932: a root and utility package without direct Vite must share the app's core and Vitest instances, and test tasks must cache.

## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp install --ignore-scripts`


## `node check-identity.mjs`

```
Root and utils have no direct Vite; app keeps its declared Vite
All packages share one Vite+, bundled Vite, and Vitest instance
```

## `cd packages/utils && vp run test`

First run populates the test cache.

```
VITE+ - The Unified Toolchain for the Web

~/packages/utils$ vp test run

 RUN  <version> <workspace>/packages/utils

 ✓ identity.test.ts (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `cd packages/utils && vp run test`

Second run must hit the cache, without tracking Vite's temporary config as an input.

```
VITE+ - The Unified Toolchain for the Web

~/packages/utils$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>/packages/utils

 ✓ identity.test.ts (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```
