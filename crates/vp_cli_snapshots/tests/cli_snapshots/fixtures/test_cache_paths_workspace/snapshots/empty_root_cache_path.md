# empty_root_cache_path

An empty runner cache path must not exclude a workspace package's source from task inputs.

## `vp run test`

```
~/packages/a$ vp test run
TAP version 13
1..1
ok 1 - src/cache.test.ts # time=<duration> {
    1..1
    ok 1 - requires the original source value # time=<duration>
}
```

## `vp run test`

Populate and confirm the task cache before changing source.

```
~/packages/a$ vp test run ◉ cache hit, replaying
TAP version 13
1..1
ok 1 - src/cache.test.ts # time=<duration> {
    1..1
    ok 1 - requires the original source value # time=<duration>
}

---
vp run: cache hit, <duration> saved.
```

## `vpt write-file src/value.ts 'export const value = 2;
'`


## `vp run test`

The source change must miss the task cache and fail the test.

**Exit code:** 1

```
~/packages/a$ vp test run ○ cache miss: 'packages/a/src/value.ts' modified, executing
TAP version 13
1..1
not ok 1 - src/cache.test.ts # time=<duration> {
    1..1
    not ok 1 - requires the original source value # time=<duration>
        ---
        error:
            name: "AssertionError"
            message: "expected 2 to be 1 // Object.is equality"
        at: "<workspace>/packages/a/src/cache.test.ts:6:17"
        actual: "2"
        expected: "1"
        ...
}
```

## `vp test run`

The direct run must report the same failure.

**Exit code:** 1

```
TAP version 13
1..1
not ok 1 - src/cache.test.ts # time=<duration> {
    1..1
    not ok 1 - requires the original source value # time=<duration>
        ---
        error:
            name: "AssertionError"
            message: "expected 2 to be 1 // Object.is equality"
        at: "<workspace>/packages/a/src/cache.test.ts:6:17"
        actual: "2"
        expected: "1"
        ...
}
```
