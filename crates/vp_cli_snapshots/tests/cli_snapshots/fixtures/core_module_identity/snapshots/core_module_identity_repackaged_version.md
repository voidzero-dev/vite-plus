# core_module_identity_repackaged_version

CI can stamp the package version after building the CLI. The resolver must compare installed manifests rather than an inlined build-time version.

## `vp install --ignore-scripts`


## `vpt json-edit node_modules/vite-plus/package.json version 0.0.0`


## `vpt json-edit node_modules/vite-plus/package.json dependencies.vite npm:@voidzero-dev/vite-plus-core@0.0.0`


## `vpt json-edit node_modules/vite/package.json version 0.0.0`


## `node check-api.mjs`

```
Packed alias, ESM, CommonJS, and module-runner identity passed
```

## `vp dev`

```

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
SSR environment identity and HTTP response passed
```

## `vp build`

```
VITE+ - The Unified Toolchain for the Web

✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp pack entry.js`

```
VITE+ - The Unified Toolchain for the Web

ℹ entry: entry.js
ℹ Build start
ℹ Cleaning <n> files
ℹ dist/entry.mjs  <size> kB │ gzip: <size> kB
ℹ 1 files, total: <size> kB
✔ Build complete in <duration>
```
