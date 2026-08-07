# create_missing_typecheck

## `vp create vite:application --no-interactive`

create standalone app


## `vpt print-file vite-plus-application/vite.config.ts`

check standalone vite.config.ts has typeAware and typeCheck

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  fmt: {},
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
});
```

## `vp create vite:monorepo --no-interactive`

create monorepo


## `vpt print-file vite-plus-monorepo/vite.config.ts`

check monorepo root vite.config.ts has typeAware and typeCheck

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  fmt: {},
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
  run: {
    cache: true,
  },
});
```

## `vpt stat-file vite-plus-monorepo/apps/website/vite.config.ts --assert-not file`

sub-app should NOT have typeAware/typeCheck

```
vite-plus-monorepo/apps/website/vite.config.ts: missing
```
