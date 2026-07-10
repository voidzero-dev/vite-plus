# vp_build_auto_tracked_env

## `VITE_GREETING=hello vp run build`

first build

```
$ vp build
vite <version> building client environment for production...
transforming...✓ 4 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `VITE_GREETING=hello vp run build`

same env, cache hit

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

## `VITE_GREETING=world vp run build`

VITE_ env changed, cache miss (tracked via vite-task-client)

```
$ vp build ○ cache miss: env 'VITE_GREETING' changed, executing
vite <version> building client environment for production...
transforming...✓ 4 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
