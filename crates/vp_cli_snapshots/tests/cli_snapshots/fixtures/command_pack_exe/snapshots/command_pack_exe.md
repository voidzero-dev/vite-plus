# command_pack_exe

## `vp pack src/index.ts --exe`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ target: node25.7.0
ℹ `exe` option is experimental and may change in future releases.
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
ℹ build/index  <size> MB
✔ Built executable: build/index (<duration>)
```

## `vpt list-dir dist`

```
index.mjs
```

## `vpt list-dir build`

```
index
```

## `./build/index`

the packed executable runs

```
hello from exe
```
