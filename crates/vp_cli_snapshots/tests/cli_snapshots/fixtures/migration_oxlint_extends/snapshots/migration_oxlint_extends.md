# migration_oxlint_extends

## `node verify-lint.mjs --before`

Original inherited errors, warnings, and overrides

```
vp lint -c .oxlintrc.json: 3 errors and 2 warnings
example.test.js: error override(no-bar)
example.test.js: warning eslint(no-debugger)
index.js: error eslint(no-debugger)
index.js: error local(no-foo)
index.js: warning eslint(no-console)
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 2 config updates applied
```

## `vpt print-file vite.config.ts`

Path extends become config objects; relative plugins move to the root lint config

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
  lint: {
    extends: [
      {
        extends: [
          {
            options: {
              typeAware: false,
              typeCheck: false,
            },
            rules: {
              "no-debugger": "error",
              "no-console": "error",
              "no-alert": "error",
            },
          },
        ],
        overrides: [
          {
            files: ["**/*.test.js"],
            rules: {
              "no-debugger": "warn",
              "override/no-bar": "error",
            },
          },
        ],
      },
      {
        rules: {
          "no-console": "warn",
        },
      },
    ],
    rules: {
      "no-alert": "off",
      "local/no-foo": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    ignorePatterns: ["shared/**", "verify-lint.mjs"],
    jsPlugins: [
      {
        name: "override",
        specifier: "./shared/override-plugin.mjs",
      },
      "./shared/plugin.mjs",
      {
        name: "vite-plus",
        specifier: "vite-plus/oxlint-plugin",
      },
    ],
    options: {},
  },
});
```

## `vpt stat-file .oxlintrc.json --assert-not file`

```
.oxlintrc.json: missing
```

## `vpt print-file shared/second.jsonc`

Shared JSON configs are retained

```
{
  // Later entries take priority.
  "rules": { "no-console": "warn" },
  "jsPlugins": ["./plugin.mjs"],
}
```

## `vp exec tsc --noEmit`

Generated config types accept the resolved inheritance

```
```

## `node verify-lint.mjs`

Plain vp lint preserves inherited severity and override matching

```
vp lint: 3 errors and 2 warnings
example.test.js: error override(no-bar)
example.test.js: warning eslint(no-debugger)
index.js: error eslint(no-debugger)
index.js: error local(no-foo)
index.js: warning eslint(no-console)
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

A second migration leaves the config unchanged

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  fmt: {},
  lint: {
    extends: [
      {
        extends: [
          {
            options: {
              typeAware: false,
              typeCheck: false,
            },
            rules: {
              "no-debugger": "error",
              "no-console": "error",
              "no-alert": "error",
            },
          },
        ],
        overrides: [
          {
            files: ["**/*.test.js"],
            rules: {
              "no-debugger": "warn",
              "override/no-bar": "error",
            },
          },
        ],
      },
      {
        rules: {
          "no-console": "warn",
        },
      },
    ],
    rules: {
      "no-alert": "off",
      "local/no-foo": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    ignorePatterns: ["shared/**", "verify-lint.mjs"],
    jsPlugins: [
      {
        name: "override",
        specifier: "./shared/override-plugin.mjs",
      },
      "./shared/plugin.mjs",
      {
        name: "vite-plus",
        specifier: "vite-plus/oxlint-plugin",
      },
    ],
    options: {},
  },
});
```

## `node verify-lint.mjs`

Inherited behavior survives the second migration

```
vp lint: 3 errors and 2 warnings
example.test.js: error override(no-bar)
example.test.js: warning eslint(no-debugger)
index.js: error eslint(no-debugger)
index.js: error local(no-foo)
index.js: warning eslint(no-console)
```
