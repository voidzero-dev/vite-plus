# command_init_inline_config

## `vp lint --init`

```
Added 'lint' to 'vite.config.ts'.
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: { typeAware: true, typeCheck: true },
  },
});
```

## `vpt stat-file .oxlintrc.json --assert-not file`

check .oxlintrc.json is removed

```
.oxlintrc.json: missing
```

## `vpt rm vite.config.ts`

```
```

## `vp fmt --init`

```
Added 'fmt' to 'vite.config.ts'.
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {
    ignorePatterns: [],
  },
});
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

check .oxfmtrc.json is removed

```
.oxfmtrc.json: missing
```
