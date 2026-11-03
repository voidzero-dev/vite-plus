# migration_oxlint_extends_ts_check_pragma

## `vpt write-file vite.config.mjs '// @ts-check
/** @type {string} */
export const value = 1;
export default {};
'`


## `vpt json-edit tsconfig.json compilerOptions.allowJs true`


## `vpt json-edit tsconfig.json include '["vite.config.mjs"]'`


## `vp install`


## `vp exec tsc --noEmit`

The file-level pragma enables the original type diagnostic

**Exit code:** 2

```
vite.config.mjs:3:14 - error TS2322: Type 'number' is not assignable to type 'string'.

3 export const value = 1;
               ~~~~~

Found 1 error in vite.config.mjs:3
```

## `vp migrate --no-interactive --no-agent --no-editor --no-hooks`

```
VITE+ - The Unified Toolchain for the Web

▲ No package manager is declared in package.json; using pnpm for this migration without adding a pin. Run `vp env pin` to declare it explicitly.
● Formatting code...
● Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
```

## `vpt print-file vite.config.mjs`

Generated imports follow the file-level pragma and precede the statement's type annotation

```
// @ts-check
import oxlintConfig1 from "./shared/nested/first.js";
import oxlintConfig2 from "./shared/second.js";

/** @type {string} */
export const value = 1;
export default {
  fmt: {},
  lint: {
    extends: [oxlintConfig1, oxlintConfig2],
    rules: {
      "no-alert": "off",
      "local/no-foo": "error",
      "vite-plus/prefer-vite-plus-imports": "error",
    },
    ignorePatterns: ["shared/**", "vite.config.ts"],
    options: {},
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
  },
};
```

## `vp exec tsc --noEmit`

The pragma still enables the same type diagnostic after migration

**Exit code:** 2

```
vite.config.mjs:6:14 - error TS2322: Type 'number' is not assignable to type 'string'.

6 export const value = 1;
               ~~~~~

Found 1 error in vite.config.mjs:6
```

## `vpt replace-file-content vite.config.mjs @ts-check @ts-nocheck`

```
```

## `vpt json-edit tsconfig.json compilerOptions.checkJs true`

```
```

## `vp exec tsc --noEmit`

A leading ts-nocheck pragma can still disable project-level JavaScript checking

```
```
