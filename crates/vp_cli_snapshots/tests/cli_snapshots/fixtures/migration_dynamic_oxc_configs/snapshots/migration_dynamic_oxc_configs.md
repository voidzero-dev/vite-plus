# migration_dynamic_oxc_configs

## `vp migrate --no-interactive`

migration should import dynamic Oxc configs into Vite+

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied, 2 files had imports rewritten
```

## `vpt print-file oxlint.config.ts`

check oxlint config and helper import

```
import { defineConfig } from 'vite-plus/lint';

export default defineConfig({
  rules: {
    eqeqeq: 'error',
  },
});
```

## `vpt print-file oxfmt.config.mts`

check oxfmt config and helper import

```
import { defineConfig } from 'vite-plus/fmt';

export default defineConfig({
  printWidth: 100,
});
```

## `vpt print-file vite.config.ts`

check dynamic configs imported into vite config

```
import oxfmtConfig from './oxfmt.config.mjs';

import oxlintConfig from './oxlint.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: oxfmtConfig,
  lint: oxlintConfig,
});
```

## `vpt print-file package.json`

check bundled Oxc dependencies removed

```
{
  "name": "migration-dynamic-oxc-configs",
  "scripts": {
    "lint": "vp lint",
    "format": "vp fmt --write",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vp migrate --no-interactive`

run migration again to check idempotency

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

check vite config remains unchanged

```
import oxfmtConfig from './oxfmt.config.mjs';

import oxlintConfig from './oxlint.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: oxfmtConfig,
  lint: oxlintConfig,
});
```
