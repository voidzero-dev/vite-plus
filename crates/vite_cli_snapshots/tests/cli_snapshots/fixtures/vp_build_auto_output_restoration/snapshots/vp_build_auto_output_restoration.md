# vp_build_auto_output_restoration

## `vp run build`

first build populates the cache


## `vpt list-dir dist`

build output exists

```
assets
index.html
```

## `vpt rm -rf dist`

remove the build output

```
```

## `vp run build`

rebuild hits cache

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

## `vpt list-dir dist`

dist auto-restored on cache hit (no synthetic output config)

```
assets
index.html
```
