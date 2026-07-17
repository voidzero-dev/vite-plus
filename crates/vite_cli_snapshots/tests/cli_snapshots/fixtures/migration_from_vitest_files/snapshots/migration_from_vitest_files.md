# migration_from_vitest_files

## `vp migrate --no-interactive`

migration should rewrite imports to vite-plus

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-from-vitest-files",
  "scripts": {
    "test:run": "vp test run",
    "test:ui": "vp test --ui",
    "test:coverage": "vp test run --coverage",
    "test:watch": "vp test --watch",
    "test": "vp test",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@vitest/browser-playwright": "catalog:",
    "vite": "catalog:",
    "vitest": "catalog:",
    "playwright": "*",
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

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vitest: <version>
  vite-plus: <version>
  '@vitest/browser-playwright': <version>
overrides:
  vite: 'catalog:'
  vitest: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
    - vitest
  allowedVersions:
    vite: '*'
    vitest: '*'
```

## `vpt print-file test/hello.ts`

check test/hello.ts

```
import { server } from 'vite-plus/test/browser/context';
import { test, describe, expect, it } from 'vite-plus/test';

const { readFile } = server.commands;

describe('Hello', () => {
  it('should return the correct result', () => {
    expect(true).toBe(true);
  });
});
```
