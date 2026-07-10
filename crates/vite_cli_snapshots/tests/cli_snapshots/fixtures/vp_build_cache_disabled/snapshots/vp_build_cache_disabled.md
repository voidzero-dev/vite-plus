# vp_build_cache_disabled

## `vp run app#build`

first build


## `vpt list-dir packages/app/dist`

should have the build output

```
assets
index.html
```

## `vp run app#build`

cache disabled, no cache hit

```
~/packages/app$ vp build ⊘ cache disabled
vite <version> building client environment for production...
✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp run app#build`

should show cache disabled

```
~/packages/app$ vp build ⊘ cache disabled
vite <version> building client environment for production...
✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
