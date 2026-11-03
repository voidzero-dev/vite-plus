# migration_prettier_rerun

## `vp migrate --full --no-interactive`

should detect vite-plus + prettier and auto-migrate prettier

```
VITE+ - The Unified Toolchain for the Web

● Prettier configuration detected. Auto-migrating to Oxfmt...
◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
• 2 config updates applied
• Prettier migrated to Oxfmt
• Package manager settings configured
```

## `vpt print-file package.json`

check prettier removed from devDependencies and scripts rewritten

```
{
  "name": "migration-prettier-rerun",
  "scripts": {
    "format": "vp fmt .",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite-plus": "catalog:"
  }
}
```

## `vpt stat-file .prettierrc.json --assert-not file`

check prettier config is removed

```
.prettierrc.json: missing
```

## `vpt print-file vite.config.ts`

check oxfmt config merged into vite.config.ts

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {
    semi: true,
    singleQuote: true,
    printWidth: 80,
    sortPackageJson: false,
    ignorePatterns: [],
  },
});
```
