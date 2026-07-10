# vp_build_cache_monorepo

## `vp run app#build`

should build the app from root


## `vpt list-dir packages/app/dist`

should have the build output

```
assets
index.html
```

## `vp run app#build`

should hit cache from root

```
~/packages/app$ vp build ◉ cache hit, replaying
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

## `cd packages/app && vp run build`

should hit cache from sub dir

```
~/packages/app$ vp build ◉ cache hit, replaying
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

## `cd packages/app && vp build`

direct vp build should not be cached


## `cd packages/app && vp build`

direct vp build has no cache

```
vite <version> building client environment for production...
✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vpt write-file packages/app/index.html '<html><body><script type="module">console.log("changed");</script></body></html>
'`

```
```

## `vp run app#build`

should miss cache after source change

```
~/packages/app$ vp build ○ cache miss: 'packages/app/index.html' modified, executing
vite <version> building client environment for production...
transforming...✓ 4 modules transformed.
rendering chunks...
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `cd packages/web && vp run build`

should build from sub dir first


## `vpt list-dir packages/web/dist`

should have the build output

```
assets
index.html
```

## `cd packages/web && vp run build`

should hit cache from sub dir

```
~/packages/web$ vp build ◉ cache hit, replaying
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

## `vp run web#build`

should hit cache from root after sub dir build

```
~/packages/web$ vp build ◉ cache hit, replaying
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
