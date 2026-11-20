# migration_import_rule_template_dependency

Only the package whose source still imports Vitest needs a direct Vitest dependency.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file templates/starter/demo.test.ts`

```
import { test } from 'vitest';

test('starter', () => {});
```

## `vpt print-file templates/starter/package.json`

```
{
  "name": "starter",
  "private": true,
  "type": "module",
  "devDependencies": {
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file packages/app/demo.test.ts`

```
import { test } from 'vite-plus/test';

test('app', () => {});
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "private": true,
  "type": "module",
  "devDependencies": {
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

```
packages:
  - templates/*
  - packages/*
nodeLinker: isolated
hoist: false
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>
overrides:
  vite@*: 'catalog:'
  vitest@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

## `vpt stat-file templates/starter/node_modules --assert missing`

```
templates/starter/node_modules: missing
```

## `vp install --ignore-scripts`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file templates/starter/package.json`

```
{
  "name": "starter",
  "private": true,
  "type": "module",
  "devDependencies": {
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt stat-file templates/starter/node_modules/vitest/package.json --assert file`

```
templates/starter/node_modules/vitest/package.json: file
```

## `cd templates/starter && node --input-type=module -e 'import { test } from '\''vitest'\''; console.log(typeof test);'`

```
function
```
