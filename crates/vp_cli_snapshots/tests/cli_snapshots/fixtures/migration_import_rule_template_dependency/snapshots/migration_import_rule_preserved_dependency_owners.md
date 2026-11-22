# migration_import_rule_preserved_dependency_owners

An unmanaged template resolves Vitest from its workspace root, and a managed template retains its browser packages while the app uses Vite+.

## `vpt replace-file-content pnpm-workspace.yaml templates/* templates/starter`


## `vpt json-edit package.json devDependencies.vitest ^5.0.0`


## `vpt write-file templates/unmanaged/package.json '{"name":"unmanaged-template","private":true}'`


## `vpt cp templates/starter/demo.test.ts templates/unmanaged/demo.test.ts`


## `vpt json-edit templates/starter/package.json devDependencies.@vitest/browser ^5.0.0`


## `vpt json-edit templates/starter/package.json devDependencies.@vitest/browser-preview ^5.0.0`


## `vpt write-file templates/starter/browser.test.ts 'import { page } from '\''@vitest/browser/context'\'';
import '\''@vitest/browser-preview'\'';
export { page };
'`


## `vpt json-edit packages/app/package.json devDependencies.@vitest/browser ^5.0.0`


## `vpt json-edit packages/app/package.json devDependencies.@vitest/browser-preview ^5.0.0`


## `vpt cp templates/starter/browser.test.ts packages/app/browser.test.ts`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file templates/unmanaged/demo.test.ts package.json`

```
import { test } from 'vitest';

test('starter', () => {});
{
  "devDependencies": {
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "import-rule-workspace",
  "packageManager": "pnpm@12.11.2",
  "private": true,
  "type": "module"
}
```

## `vpt print-file templates/starter/browser.test.ts templates/starter/package.json`

```
import { page } from '@vitest/browser/context';
import '@vitest/browser-preview';
export { page };
{
  "devDependencies": {
    "@vitest/browser": "catalog:",
    "@vitest/browser-preview": "catalog:",
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "starter",
  "private": true,
  "type": "module"
}
```

## `vpt print-file packages/app/demo.test.ts packages/app/browser.test.ts packages/app/package.json`

```
import { test } from 'vite-plus/test';

test('app', () => {});
import { page } from 'vite-plus/test/browser/context';
import 'vite-plus/test/browser-preview';
export { page };
{
  "devDependencies": {
    "vitest": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "app",
  "private": true,
  "type": "module"
}
```

## `vp install --ignore-scripts`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vp install --frozen-lockfile --ignore-scripts`


## `vpt stat-file node_modules/vitest/package.json templates/starter/node_modules/@vitest/browser/package.json templates/starter/node_modules/@vitest/browser-preview/package.json --assert file`

```
node_modules/vitest/package.json: file
templates/starter/node_modules/@vitest/browser/package.json: file
templates/starter/node_modules/@vitest/browser-preview/package.json: file
```

## `vpt stat-file templates/unmanaged/node_modules/vitest/package.json packages/app/node_modules/@vitest/browser/package.json packages/app/node_modules/@vitest/browser-preview/package.json --assert missing`

```
templates/unmanaged/node_modules/vitest/package.json: missing
packages/app/node_modules/@vitest/browser/package.json: missing
packages/app/node_modules/@vitest/browser-preview/package.json: missing
```

## `cd templates/unmanaged && node --input-type=module -e 'import { statSync } from '\''node:fs'\''; console.log(statSync(new URL(import.meta.resolve('\''vitest'\''))).isFile());'`

```
true
```

## `cd templates/starter && node --input-type=module -e 'import { statSync } from '\''node:fs'\''; for (const name of process.argv.slice(1)) console.log(name, statSync(new URL(import.meta.resolve(name))).isFile());' @vitest/browser/context @vitest/browser-preview`

```
@vitest/browser/context true
@vitest/browser-preview true
```
