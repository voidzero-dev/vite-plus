# migration_upgrade_hooks_standalone_staged_pnpm

## `git init`


## `vpt json-edit package.json devDependencies.lint-staged '"^16.2.7"'`


## `vpt write-file .lintstagedrc.json '{"*.ts":"eslint --fix"}
'`


## `vp migrate --hooks --no-interactive`

migrate hooks and staged config

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
    vitest     0.1.21 → <version>
• 3 config updates applied
• Git hooks configured
• Package manager settings configured
```

## `git config --local core.hooksPath`

install the dispatcher

```
.vite-hooks/_
```

## `vpt stat-file .lintstagedrc.json --assert missing`


## `vpt print-file .vite-hooks/pre-commit`

add the staged hook

```
vp staged
```

## `vpt grep-file vite.config.ts staged:`


## `vpt print-file vite.config.ts`

inline staged config

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*.ts": "eslint --fix"
  },

});
```

## `vpt print-file package.json`

remove lint-staged

```
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:",
    "vitest": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "onFail": "download",
      "version": "10.33.0"
    }
  },
  "name": "migration-upgrade-hooks-flag-pnpm",
  "scripts": {
    "prepare": "vp config"
  }
}
```
