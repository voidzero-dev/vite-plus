# vp_build_cache

## `vp run build`

first build


## `vp run build`

should hit cache

```
$ vp build ◉ cache hit, replaying
vite <version> building client environment for production...
transforming...✓ 4 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>

---
vp run: cache hit, <duration> saved.
```

## `vp build`

direct vp build should not be cached


## `vp build`

direct vp build has no cache

```
vite <version> building client environment for production...
✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
