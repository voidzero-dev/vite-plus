# vp_fmt_cache

## `vp run fmt`

first run populates the cache

```
$ vp fmt
Finished in <duration> on 3 files using <n> threads.
```

## `vp run fmt`

second run should be a cache hit

```
$ vp fmt ◉ cache hit, replaying
Finished in <duration> on 3 files using <n> threads.

---
vp run: cache hit, <duration> saved.
```
