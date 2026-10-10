# create_workspace_legacy_pack

## `vp create library --no-interactive --no-hooks --no-agent --no-editor`


## `vpt print-file packages/library/vite.config.ts`

```
import { defineConfig } from "vite-plus";
export default defineConfig({
  pack: {
    deps: {
      // tsdown <0.23 compatibility: resolve external dependency subpaths.
      // Remove to preserve subpath imports as written (the new default).
      // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
      resolveDepSubpath: true,
    },
    dts: { generator: "tsgo" },
    exports: true,
  },
});
```

## `cd packages/library && vp run build`

```
~/packages/library$ vp pack ⊘ cache disabled
ℹ entry: src/index.ts
ℹ tsconfig: tsconfig.json
ℹ Build start
warn: TypeScript 7.0 does not yet have a stable API and is experimental. Some options will be unavailable.
ℹ Emit types with @typescript/native-preview@7.0.0-dev.20260605.1
ℹ dist/index.mjs    <size> kB │ gzip: <size> kB
ℹ dist/index.d.mts  <size> kB │ gzip: <size> kB
ℹ 2 files, total: <size> kB
✔ Build complete in <duration>
```

## `vpt stat-file packages/library/dist/index.mjs --assert file`

```
packages/library/dist/index.mjs: file
```

## `vpt stat-file packages/library/dist/index.d.mts --assert file`

```
packages/library/dist/index.d.mts: file
```
