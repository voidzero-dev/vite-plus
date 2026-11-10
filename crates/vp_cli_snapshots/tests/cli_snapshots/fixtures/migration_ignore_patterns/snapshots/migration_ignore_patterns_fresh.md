# migration_ignore_patterns_fresh

Initial migration honors the same exclusions.

## `vpt json-edit package.json devDependencies {}`

```
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 3 files had imports rewritten
```

## `vpt print-file templates/starter/package.json`

```
{
  "name": "starter", "private": true, "type": "module",
  "scripts": { "build": "vite build", "test": "vitest run" },
  "devDependencies": { "vite": "catalog:", "vitest": "catalog:legacy", "@vitest/coverage-v8": "catalog:legacy" }
}
```

## `vpt print-file templates/starter/vite.config.ts`

```
import { defineConfig } from 'vite';
export default defineConfig({test: {poolOptions: {threads: {singleThread: true}}}});
```

## `vpt print-file templates/starter/unit.test.ts`

```
import { expect, test } from 'vitest';
debugger;
test('template', () => expect(1).toBe(1));
```

## `vpt print-file templates/starter/tsconfig.json`

```
{ "compilerOptions": { "types": ["vite/client", "vitest/globals"], "esModuleInterop": false } }
```

## `vpt print-file examples/standalone/unit.test.ts`

```
import { expect, test } from 'vitest';
test('outside-workspace', () => expect(1).toBe(1));
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vp build",
    "test": "vp test run"
  },
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>",
    "@vitest/coverage-v8": "<version>",
    "vite-plus": "<version>"
  }
}
```

## `vpt print-file packages/app/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('application', () => expect(1).toBe(1));
```

## `vpt print-file templates/managed/unit.test.ts`

```
import { expect, test } from 'vite-plus/test';
test('included', () => expect(1).toBe(1));
```

## `vpt print-file pnpm-workspace.yaml`

```
packages:
  - packages/*
  - templates/*
catalog:
  vite-plus: latest
  vite: ^8.0.0
catalogs:
  legacy:
    vitest: ^4.1.11
    '@vitest/coverage-v8': ^4.1.11
overrides:
  unrelated-package: 1.0.0
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```
