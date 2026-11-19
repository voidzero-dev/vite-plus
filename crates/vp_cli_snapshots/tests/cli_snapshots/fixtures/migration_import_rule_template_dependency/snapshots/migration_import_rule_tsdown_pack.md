# migration_import_rule_tsdown_pack

Both libraries build with vp pack after migration. The template keeps its Vitest import.

## `vpt json-edit templates/starter/package.json devDependencies.tsdown ^0.23.0`


## `vpt json-edit packages/app/package.json devDependencies.tsdown ^0.23.0`


## `vpt write-file templates/starter/tsdown.config.mjs 'import { defineConfig } from '\''tsdown'\'';
export default defineConfig({ entry: ['\''index.ts'\''], fixedExtension: true });
'`


## `vpt cp templates/starter/tsdown.config.mjs packages/app/tsdown.config.mjs`


## `vpt write-file templates/starter/index.ts 'export const answer = 42;
'`


## `vpt cp templates/starter/index.ts packages/app/index.ts`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied, 3 files had imports rewritten
→ Manual follow-up:
  - Please manually merge packages/app/tsdown.config.mjs into packages/app/vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
  - Please manually merge templates/starter/tsdown.config.mjs into templates/starter/vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
```

## `vpt print-file templates/starter/tsdown.config.mjs templates/starter/demo.test.ts`

```
import { defineConfig } from 'vite-plus/pack';
export default defineConfig({ deps: {
    // tsdown <0.23 compatibility: resolve external dependency subpaths.
    // Remove to preserve subpath imports as written (the new default).
    // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
    resolveDepSubpath: true }, entry: ['index.ts'], fixedExtension: true });
import { test } from 'vitest';

test('starter', () => {});
```

## `vpt print-file packages/app/tsdown.config.mjs packages/app/demo.test.ts`

```
import { defineConfig } from 'vite-plus/pack';
export default defineConfig({ deps: {
    // tsdown <0.23 compatibility: resolve external dependency subpaths.
    // Remove to preserve subpath imports as written (the new default).
    // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
    resolveDepSubpath: true }, entry: ['index.ts'], fixedExtension: true });
import { test } from 'vite-plus/test';

test('app', () => {});
```

## `vpt stat-file templates/starter/node_modules packages/app/node_modules --assert missing`

```
templates/starter/node_modules: missing
packages/app/node_modules: missing
```

## `vp install --ignore-scripts`


## `vpt stat-file templates/starter/node_modules/tsdown packages/app/node_modules/tsdown --assert missing`

```
templates/starter/node_modules/tsdown: missing
packages/app/node_modules/tsdown: missing
```

## `cd packages/app && vp pack`


## `vpt stat-file packages/app/dist/index.mjs --assert file`

```
packages/app/dist/index.mjs: file
```

## `cd templates/starter && vp pack`


## `vpt stat-file templates/starter/dist/index.mjs --assert file`

```
templates/starter/dist/index.mjs: file
```
