# check_cache_chdir

A leading -C keeps the cache config of vp check, so its tracked env still invalidates the cache (#3503).

## `vp run check:chdir`

first run should be cache miss

```
$ vp -C . check
pass: All 3 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)
```

## `VP_SKIP_STATIC_CONFIG=1 vp run check:chdir`

changing VP_SKIP_STATIC_CONFIG should be a cache miss

```
$ vp -C . check ○ cache miss: env 'VP_SKIP_STATIC_CONFIG' changed, executing
pass: All 3 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)
```
