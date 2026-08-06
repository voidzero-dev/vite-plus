# check_cache_enabled

## `vp run check`

first run should be cache miss

```
$ vp check
pass: All 3 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)
```

## `vp run check`

second run should be cache hit

```
$ vp check ◉ cache hit, replaying
pass: All 3 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)

---
vp run: cache hit, <duration> saved.
```

## `vpt write-file src/foo.js 'export const foo = 1;
'`

```
```

## `vp run check`

third run should be cache miss after new file added

```
$ vp check ○ cache miss: 'foo.js' added in 'src', executing
pass: All 4 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 3 files (<duration>, <n> threads)
```
