# command_pack_monorepo

## `vp run hello#build`

should build the library from root


## `vpt list-dir packages/hello/dist`

should have the library

```
index.cjs
```

## `vp run hello#build`

should hit cache from root

```
~/packages/hello$ vp pack ◉ cache hit, replaying
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.cjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```

## `cd packages/hello && vp run build`

should hit cache from sub dir

```
~/packages/hello$ vp pack ◉ cache hit, replaying
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.cjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```

## `cd packages/hello && vp pack`

direct vp pack should not be cached


## `cd packages/hello && vp pack`

direct vp pack has no cache

```
ℹ entry: src/index.ts
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.cjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt write-file packages/hello/src/hello.ts 'export function hello() { console.log("changed"); }
'`

```
```

## `vp run hello#build`

should miss cache after source change

```
~/packages/hello$ vp pack ○ cache miss: 'packages/hello/src/hello.ts' modified, executing
ℹ entry: src/index.ts
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.cjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `cd packages/array-config && vp run build`

should build the library from sub dir


## `vpt list-dir packages/array-config/dist`

should have the library

```
index.d.mts
index.mjs
```

## `cd packages/array-config && vp run build`

should hit cache from sub dir

```
~/packages/array-config$ vp pack ◉ cache hit, replaying
ℹ entry: ./src/sub/index.ts
ℹ Build start
ℹ dist/index.mjs    <size> kB │ gzip: <size> kB
ℹ dist/index.d.mts  <size> kB │ gzip: <size> kB
ℹ 2 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```

## `vp run array-config#build`

should hit cache from root after sub dir build

```
~/packages/array-config$ vp pack ◉ cache hit, replaying
ℹ entry: ./src/sub/index.ts
ℹ Build start
ℹ dist/index.mjs    <size> kB │ gzip: <size> kB
ℹ dist/index.d.mts  <size> kB │ gzip: <size> kB
ℹ 2 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```

## `vp run default-config#build`

should build the library from root


## `vpt list-dir packages/default-config/dist`

should have the library

```
index.mjs
```

## `cd packages/default-config && vp run build`

should hit cache from sub dir

```
~/packages/default-config$ vp pack ◉ cache hit, replaying
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```
