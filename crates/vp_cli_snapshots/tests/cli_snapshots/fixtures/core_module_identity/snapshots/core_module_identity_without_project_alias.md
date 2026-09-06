# core_module_identity_without_project_alias

## `vpt write-file package.json '{"name":"core-module-identity","private":true,"type":"module","devDependencies":{"vite-plus":"latest"},"packageManager":"pnpm@11.24.0"}
'`


## `vpt write-file pnpm-workspace.yaml 'hoist: false
minimumReleaseAge: 0
overrides:
  vite: npm:@voidzero-dev/vite-plus-core@latest
'`


## `vpt replace-file-content vite.config.ts 'from '\''vite'\'';' 'from '\''vite-plus'\'';'`


## `vp install --ignore-scripts`


## `node check-bundled.mjs`

```
Bundled APIs resolve without a project Vite dependency
```

## `vp dev`

```

  VITE <version>

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

## `vp test run`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓ identity.test.js (1 test) <duration>
   ✓ the public guard accepts an environment created by the bundled server <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)
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
