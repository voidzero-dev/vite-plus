# plugin_projects_and_dom_types

Do not scan plugin project paths as Vitest configs; report the Jest-only DOM type entry without changing a mixed test contract.

## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vitest/config'\'';
const config = defineConfig({ plugins: [{ name: '\''paths'\'', projects: ['\''./tsconfig.json'\'', '\''./plugin-options.ts'\''] }], test: {} });
export default config;
'`


## `vpt write-file tsconfig.json '{"compilerOptions":{"types":["@testing-library/jest-dom","vitest/globals"]}}
'`


## `vpt write-file plugin-options.ts 'export default { test: {} };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 2 review items

tsconfig.json
  1:1 REVIEW [jest-dom-types] If this config checks Vitest tests, load @testing-library/jest-dom/vitest in compilerOptions.types or an included TypeScript setup file. The root jest-dom type entry augments Jest, not Vitest v5.

vite.config.ts
  3:1 REVIEW [dynamic-config] Review the effective exported test config and its v4 defaults.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied, 2 files had imports rewritten
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
! Warnings:
  - Vitest v5: 2 review items

tsconfig.json
  1:1 REVIEW [jest-dom-types] If this config checks Vitest tests, load @testing-library/jest-dom/vitest in compilerOptions.types or an included TypeScript setup file. The root jest-dom type entry augments Jest, not Vitest v5.

vite.config.ts
  7:1 REVIEW [dynamic-config] Review the effective exported test config and its v4 defaults.
```

## `vpt print-file plugin-options.ts`

```
export default { test: {} };
```

## `vpt print-file tsconfig.json`

```
{"compilerOptions":{"types":["@testing-library/jest-dom","vitest/globals"]}}
```
