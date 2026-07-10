# synthetic_build_cache_disabled

## `vp run build`

synthetic build (vp build) should have cache disabled without cacheScripts

```
$ vp build ⊘ cache disabled
vite <version> building client environment for production...
✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
