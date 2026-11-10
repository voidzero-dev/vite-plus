# migration_ignore_patterns_override_conflict

A conflicting global override is reported before project files change.

## `vpt write-file pnpm-workspace.yaml 'packages: ['\''packages/*'\'', '\''templates/*'\'']
overrides:
  vite: ^8.0.0
'`

```
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

■ Cannot migrate while preserving excluded packages: override "vite" ("^8.0.0") conflicts with "npm:@voidzero-dev/vite-plus-core@<version>". Review the root override or migrate the excluded packages first. No project files were changed.
Error: Cannot migrate while preserving excluded packages: override "vite" ("^8.0.0") conflicts with "npm:@voidzero-dev/vite-plus-core@<version>". Review the root override or migrate the excluded packages first. No project files were changed.
```

## `vpt print-file packages/app/package.json`

```
{
  "name": "app", "private": true, "type": "module",
  "scripts": { "build": "vite build", "test": "vitest run" },
  "devDependencies": { "vite": "catalog:", "vitest": "catalog:legacy", "@vitest/coverage-v8": "catalog:legacy" }
}
```

## `vpt print-file packages/app/unit.test.ts`

```
import { expect, test } from 'vitest';
test('application', () => expect(1).toBe(1));
```

## `vpt print-file pnpm-workspace.yaml`

```
packages: ['packages/*', 'templates/*']
overrides:
  vite: ^8.0.0
```
