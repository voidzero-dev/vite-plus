# command_pack

## `vp pack -h`

should print the help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pack [...FILES] [OPTIONS]

Build a library.
Options are forwarded to Vite+ Pack.

Options:
  --config-loader <LOADER>  Set the config loader
  --no-config               Disable the config file
  -f, --format <FORMAT>     Bundle format: esm, cjs, iife, umd
  -d, --out-dir <DIR>       Output directory
  --target <TARGET>         Bundle target
  --platform <PLATFORM>     Target platform
  --sourcemap               Generate source maps
  --dts                     Generate declaration files
  --minify                  Minify output
  --exe                     Bundle as an executable
  -W, --workspace [DIR]     Enable workspace mode
  -F, --filter <PATTERN>    Filter workspace configs
  -w, --watch [PATH]        Watch mode
  -h, --help                Print help

Examples:
  vp pack
  vp pack src/index.ts --dts
  vp pack --watch

Documentation: https://viteplus.dev/guide/pack
```

## `vp run pack`

should build the library

```
$ vp pack src/index.ts
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt list-dir dist`

should have the library

```
index.mjs
```

## `vp run pack`

should hit cache

```
$ vp pack src/index.ts ◉ cache hit, replaying
ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>

---
vp run: cache hit, <duration> saved.
```
