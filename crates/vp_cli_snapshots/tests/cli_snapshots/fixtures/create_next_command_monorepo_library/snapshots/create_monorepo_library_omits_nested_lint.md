# create_monorepo_library_omits_nested_lint

## `vp create vite:library --no-interactive`

create a library in an existing monorepo


## `vpt print-file packages/vite-plus-library/vite.config.ts`

nested library config should omit lint and fmt

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
    dts: {
      generator: "tsgo",
    },
    exports: true,
  },
});
```
