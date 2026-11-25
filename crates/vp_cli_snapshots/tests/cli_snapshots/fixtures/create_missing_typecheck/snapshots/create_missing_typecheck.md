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
import cloudflare from "@cloudflare/oxlint-config/vite-plus";
import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix",
  },
  fmt: {},
  lint: {
    extends: [cloudflare],
    env: cloudflare.env,
    ignorePatterns: cloudflare.ignorePatterns,
    options: { typeCheck: true },
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: {
      "no-console": ["error", { allow: ["error"] }],
      "typescript/no-non-null-assertion": "off",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
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

## `vpt print-file vite-plus-monorepo/packages/utils/vite.config.ts`

sub-library should NOT have nested lint or fmt config

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
