# migration_hooks_commented_staged_key

A commented-out `staged:` must not count as existing staged config, or package.json lint-staged is deleted without being merged.

## `git init`


## `vp migrate --no-interactive --hooks`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 1 config update applied
• Git hooks configured
• Package manager settings configured
```

## `vpt print-file package.json`

lint-staged should be removed only because it was merged

```
{
  "name": "migration-hooks-commented-staged-key",
  "devDependencies": {
    "vite-plus": "<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "npm",
      "version": "<version>",
      "onFail": "download"
    }
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "scripts": {
    "prepare": "vp config"
  }
}
```

## `vpt print-file vite.config.ts`

staged config should be merged from package.json

```
import { defineConfig } from 'vite-plus';

// TODO: move lint-staged here as staged: { ... }
export default defineConfig({
  staged: {
    "*.ts": "vp check --fix"
  },

});
```
