# vp_lint_cache

## `vp run lint`

first run populates the cache

```
$ vp lint
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```

## `vp run lint`

second run should be a cache hit

```
$ vp lint ◉ cache hit, replaying
Found 0 warnings and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.

---
vp run: cache hit, <duration> saved.
```
