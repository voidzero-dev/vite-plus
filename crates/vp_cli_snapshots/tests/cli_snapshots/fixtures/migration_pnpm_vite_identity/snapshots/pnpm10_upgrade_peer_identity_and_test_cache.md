# pnpm10_upgrade_peer_identity_and_test_cache

The existing-Vite+ upgrade path must also preserve dependency identity and caching without injecting Vite, including on pnpm 10.

## `vpt json-edit package.json packageManager pnpm@10.33.0`


## `vp migrate --no-interactive`


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

Second run must hit the cache.

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

## `vp migrate --no-interactive`


## `node check-identity.mjs`

Repeated migration must not inject Vite either.

```
Root and utils have no direct Vite; app keeps its declared Vite
All packages share one Vite+, bundled Vite, and Vitest instance
```
