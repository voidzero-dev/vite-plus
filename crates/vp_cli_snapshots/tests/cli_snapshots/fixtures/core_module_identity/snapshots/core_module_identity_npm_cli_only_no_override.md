# core_module_identity_npm_cli_only_no_override

## `vpt write-file package.json '{"name":"core-module-identity","private":true,"type":"module","packageManager":"npm@11.11.0","devDependencies":{"vite-plus":"latest"}}
'`


## `vpt replace-file-content vite.config.ts 'from '\''vite'\'';' 'from '\''vite-plus'\'';'`


## `vp install --ignore-scripts`


## `node check-npm-layout.mjs`

```
npm installed separate CLI core and upstream Vitest peer without overrides
```

## `node check-bundled.mjs`

```
Bundled APIs resolve without a project Vite dependency
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

## `vpt write-file typecheck/package.json '{"name":"typecheck-tools","private":true,"dependencies":{"typescript":"6.0.3"},"packageManager":"npm@11.11.0"}
'`


## `cd typecheck && vp install --ignore-scripts`


## `vpt cp check-config.mts check-config.cts`


## `node typecheck/node_modules/typescript/bin/tsc --noEmit --skipLibCheck --module NodeNext --target ESNext check-config.mts check-config.cts`

```
```
