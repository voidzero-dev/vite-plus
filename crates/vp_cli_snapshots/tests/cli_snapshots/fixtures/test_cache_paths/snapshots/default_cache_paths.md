# default_cache_paths

Vitest's default disk caches are neither task inputs nor restored outputs.

## `vp run test`

Populate the task cache and Vitest's default disk caches.

```
$ vp test run

 RUN  <version> <workspace>

 ✓ src/cache.test.ts (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `node cache-files.mjs check node_modules/.vite/vitest node_modules/.vitest-cache`

```
Cache files exist in node_modules/.vite/vitest
Cache files exist in node_modules/.vitest-cache
```

## `vp run test`

The second run must hit the task cache.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓ src/cache.test.ts (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs mutate node_modules/.vite/vitest node_modules/.vitest-cache`

```
Modified cache files in node_modules/.vite/vitest
Modified cache files in node_modules/.vitest-cache
```

## `vp run test`

Changing Vitest's cache contents must not invalidate the task.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓ src/cache.test.ts (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs clear node_modules/.vite/vitest node_modules/.vitest-cache`

```
Removed cache files in node_modules/.vite/vitest
Removed cache files in node_modules/.vitest-cache
```

## `vp run test`

Removing cache files while preserving their directories must still hit the task cache.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓ src/cache.test.ts (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs absent node_modules/.vite/vitest node_modules/.vitest-cache`

```
No cache files restored in node_modules/.vite/vitest
No cache files restored in node_modules/.vitest-cache
```

## `vpt write-file src/value.ts 'export const value = 2;
'`


## `vp run test`

Changing source must miss the task cache and run Vitest again.

```
$ vp test run ○ cache miss: 'src/value.ts' modified, executing

 RUN  <version> <workspace>

 ✓ src/cache.test.ts (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `node cache-files.mjs check node_modules/.vite/vitest node_modules/.vitest-cache`

```
Cache files exist in node_modules/.vite/vitest
Cache files exist in node_modules/.vitest-cache
```
