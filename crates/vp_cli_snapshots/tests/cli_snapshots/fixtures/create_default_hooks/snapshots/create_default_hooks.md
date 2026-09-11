# create_default_hooks

## `vp create vite:application --no-interactive`

create should scaffold the default hook policy


## `vpt print-file vite-plus-application/.vite-hooks/pre-commit`

project-owned pre-commit hook should run vp staged

```
vp staged
```

## `vpt print-file vite-plus-application/vite.config.ts`

vite config should contain the matching staged policy

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
