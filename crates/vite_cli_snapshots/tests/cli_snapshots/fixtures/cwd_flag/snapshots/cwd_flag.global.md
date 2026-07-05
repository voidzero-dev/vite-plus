# cwd_flag

The global -C flag runs any command as if vp was started in the directory:
pack and run behave byte-identically to the cd forms, a missing directory
errors, and the positional keeps upstream tsdown entry semantics
(rfcs/cwd-flag.md).

## `vp -C packages/hello pack`

-C packs the package from the workspace root

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt list-dir packages/hello/dist`

output lands in the target package

```
index.mjs
```

## `vpt rm -rf packages/hello/dist`

reset so both forms produce identical output

```
```

## `cd packages/hello && vp pack`

the cd form is equivalent

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: src/index.ts
ℹ Build start
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vp -C packages/hello run where`

-C applies to vp run as well

```
VITE+ - The Unified Toolchain for the Web

~/packages/hello$ node -e "console.log('cwd base: ' + require('node:path').basename(process.cwd()))" ⊘ cache disabled
cwd base: hello
```

## `cd packages/hello && vp run where`

equivalent cd form for run

```
VITE+ - The Unified Toolchain for the Web

~/packages/hello$ node -e "console.log('cwd base: ' + require('node:path').basename(process.cwd()))" ⊘ cache disabled
cwd base: hello
```

## `vp -C packages/missing build`

missing directory errors

**Exit code:** 1

```
directory not found: packages/missing
```

## `vp pack packages/hello`

positional stays a tsdown entry resolved from the invocation directory

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: packages/hello
ℹ Build start
ℹ dist/hello.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt list-dir dist`

upstream semantics: output lands at the invocation directory

```
hello.mjs
```
