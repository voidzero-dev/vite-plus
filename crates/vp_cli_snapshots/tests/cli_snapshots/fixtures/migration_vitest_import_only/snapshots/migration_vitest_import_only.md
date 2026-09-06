# migration_vitest_import_only

## `vp migrate --no-interactive`

ordinary vitest imports should migrate without retaining direct vitest

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt print-file package.json`

direct dependency and shared pin should be removed

```
{
  "name": "migration-vitest-import-only",
  "scripts": {
    "test": "vp test",
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

## `vpt print-file example.spec.ts`

source import should use the Vite+ public surface

```
import { expect, it } from 'vite-plus/test';

it('works', () => {
  expect(true).toBe(true);
});
```

## `vpt print-file pnpm-workspace.yaml`

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
