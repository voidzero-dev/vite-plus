# injected_cache_paths

Projects injected by an async pre hook have their resolved cache paths excluded.

## `vpt cp vite.injected.config.ts vite.config.ts`


## `vp run test`

The first run must be cacheable even when a project is attached after the cache-path hook.

```
$ vp test run

 RUN  <version> <workspace>

 ✓  injected-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `node cache-files.mjs check .test-cache/injected`

```
Cache files exist in .test-cache/injected
```

## `vp run test`

Both projects must be replayed from the task cache.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓  injected-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs mutate .test-cache/injected`

```
Modified cache files in .test-cache/injected
```

## `vp run test`

Changing only the injected project's cache must still hit the task cache.

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓  injected-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs clear .test-cache/injected`

```
Removed cache files in .test-cache/injected
```

## `vp run test`

```
$ vp test run ◉ cache hit, replaying

 RUN  <version> <workspace>

 ✓  injected-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```

## `node cache-files.mjs absent .test-cache/injected`

```
No cache files restored in .test-cache/injected
```

## `vpt write-file src/value.ts 'export const value = 2;
'`


## `vp run test`

A source change must rerun both projects.

```
$ vp test run ○ cache miss: 'src/value.ts' modified, executing

 RUN  <version> <workspace>

 ✓  injected-cache  src/cache.test.ts (1 test) <duration>
 ✓  root-cache  src/cache.test.ts (1 test) <duration>

 Test Files  2 passed (2)
      Tests  2 passed (2)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `node cache-files.mjs check .test-cache/injected`

```
Cache files exist in .test-cache/injected
```
