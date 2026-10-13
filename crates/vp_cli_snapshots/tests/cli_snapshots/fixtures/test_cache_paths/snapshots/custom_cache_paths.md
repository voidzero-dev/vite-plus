# custom_cache_paths

Custom cacheDir and runner/project fsModuleCachePath values are excluded from task caching.

## `vpt cp vite.custom.config.ts vite.config.ts`


## `vp run test`

Populate caches for both projects, outside node_modules.

```
$ vp test run

 RUN  <version> <workspace>

 ✓  project-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `node cache-files.mjs check .test-cache/results/vitest .test-cache/root .test-cache/project`

```
Cache files exist in .test-cache/results/vitest
Cache files exist in .test-cache/root
Cache files exist in .test-cache/project
```

## `vp run test`

Both projects must be replayed from the task cache.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓  project-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs mutate .test-cache/results/vitest .test-cache/root .test-cache/project`

```
Modified cache files in .test-cache/results/vitest
Modified cache files in .test-cache/root
Modified cache files in .test-cache/project
```

## `vp run test`

Changes to every custom cache path must leave the task cache valid.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓  project-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `vpt rm -rf .test-cache`


## `vp run test`

A cache hit must not restore the custom Vitest caches.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓  project-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs absent .test-cache/results/vitest .test-cache/root .test-cache/project`

```
No cache files restored in .test-cache/results/vitest
No cache files restored in .test-cache/root
No cache files restored in .test-cache/project
```

## `vpt write-file src/value.ts 'export const value = 2;
'`


## `vp run test`

Source changes must rerun both projects and recreate their caches.

```
$ vp test run ○ cache miss: 'src/value.ts' modified, executing

 RUN  <version> <workspace>

 ✓  project-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `node cache-files.mjs check .test-cache/results/vitest .test-cache/root .test-cache/project`

```
Cache files exist in .test-cache/results/vitest
Cache files exist in .test-cache/root
Cache files exist in .test-cache/project
```
