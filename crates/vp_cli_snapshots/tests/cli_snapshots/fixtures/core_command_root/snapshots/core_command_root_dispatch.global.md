# core_command_root_dispatch

The workspace declares upstream Vite. Only the target app declares the matching core alias. Check positional roots, -C, defaultPackage, and task dispatch.

## `vp install --ignore-scripts`


## `node check-layout.mjs`

```
Root Vite: vite
App Vite: @voidzero-dev/vite-plus-core
CLI Vite: @voidzero-dev/vite-plus-core
```

## `vp build packages/app`

```
VITE+ - The Unified Toolchain for the Web

✓ 2 modules transformed.
computing gzip size...
packages/app/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp build --mode production --emptyOutDir packages/app`

```
VITE+ - The Unified Toolchain for the Web

✓ 2 modules transformed.
computing gzip size...
packages/app/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp dev packages/app`

```

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
The target app served HTTP 200
```

## `vp preview --strictPort packages/app`

```
VITE+ - The Unified Toolchain for the Web

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
The target app served HTTP 200
```

## `vp -C packages/app build`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp build` as a Vite+ built-in command. If you meant to run the build npm script, use `vpr build` instead.
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp build`

```
VITE+ - The Unified Toolchain for the Web

note: vp build: using packages/app (defaultPackage in vite.config.ts)
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp pack`

```
VITE+ - The Unified Toolchain for the Web

note: vp pack: using packages/app (defaultPackage in vite.config.ts)
ℹ entry: src/index.ts
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vp run --filter app build`

```
VITE+ - The Unified Toolchain for the Web

~/packages/app$ vp build ⊘ cache disabled
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp run --filter app pack`

```
VITE+ - The Unified Toolchain for the Web

~/packages/app$ vp pack ⊘ cache disabled
ℹ entry: src/index.ts
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/index.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```

## `vp build .`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Failed to resolve vite command: GenericFailure, Expected @voidzero-dev/vite-plus-core@<version>, but found vite@8.2.2 at <workspace>/node_modules/.pnpm/vite@8.2.2/node_modules/vite/package.json. Run `vp migrate` to align the Vite alias, then run `vp install`.
```

## `vp -C . pack packages/app/src/index.ts`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Failed to resolve pack command: GenericFailure, Expected @voidzero-dev/vite-plus-core@<version>, but found vite@8.2.2 at <workspace>/node_modules/.pnpm/vite@8.2.2/node_modules/vite/package.json. Run `vp migrate` to align the Vite alias, then run `vp install`.
```
