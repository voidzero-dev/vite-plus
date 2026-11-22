# migration_merge_partial_returns

Keep the standalone configs when a direct callback return cannot receive the merged settings.

## `vpt write-file vite.config.ts 'import { defineConfig } from '\''vite'\'';

export default defineConfig(({ command }) => {
  function devConfig() {
    return { server: { port: 3000 } };
  }
  if (command === '\''serve'\'') return devConfig();
  return { build: { sourcemap: true } };
});
'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
! Warnings:
  - Failed to merge .oxlintrc.json into vite.config.ts
  - Failed to merge .oxfmtrc.json into vite.config.ts
  - Vitest v5: 1 review item

vite.config.ts
  3:29 REVIEW [dynamic-config] Review this dynamic config; explicit v4 compatibility settings need manual insertion.
    Docs: https://viteplus.dev/guide/vitest-v5#resolve-migration-findings
  - Failed to merge staged config into vite.config.ts
→ Manual follow-up:
  - Please complete the merge manually and follow the instructions in the documentation: https://viteplus.dev/config/
  - Please add staged config to vite.config.ts manually, see https://viteplus.dev/guide/migrate#lint-staged
```

## `vpt print-file vite.config.ts`

neither branch receives lint or fmt

```
import { defineConfig } from 'vite-plus';

export default defineConfig(({ command }) => {
  function devConfig() {
    return { server: { port: 3000 } };
  }
  if (command === 'serve') return devConfig();
  return { build: { sourcemap: true } };
});
// vp migrate: pending .oxlintrc.json.
```

## `vpt stat-file .oxlintrc.json --assert file`

check .oxlintrc.json is preserved

```
.oxlintrc.json: file
```

## `vpt stat-file .oxfmtrc.json --assert file`

check .oxfmtrc.json is preserved

```
.oxfmtrc.json: file
```
