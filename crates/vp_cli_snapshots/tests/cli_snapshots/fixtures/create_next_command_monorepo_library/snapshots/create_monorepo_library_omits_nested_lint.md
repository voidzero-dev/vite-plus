# create_monorepo_library_omits_nested_lint

## `vp create vite:library --no-interactive`

create a library in an existing monorepo


## `vpt print-file packages/vite-plus-library/vite.config.ts`

nested library config should omit lint

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  pack: {
    dts: {
      tsgo: true,
    },
    exports: true,
  },

  fmt: {},
});
```
