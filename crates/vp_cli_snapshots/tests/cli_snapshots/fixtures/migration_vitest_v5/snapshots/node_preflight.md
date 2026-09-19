# node_preflight

Upgrade the unsupported runtime pin before installation, then keep it on rerun.

## `vpt write-file .node-version '20.19.0
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file .node-version`

```
22.18.0
```

## `vpt print-file package.json`

```
{
  "name": "migration-vitest-v5",
  "private": true,
  "type": "module",
  "packageManager": "pnpm@11.24.0",
  "scripts": {
    "test": "vp test list --no-static-parse"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  }
}
```

## `vpt print-file example.test.ts`

```
import { expect, test } from 'vite-plus/test';

test('compatibility', { concurrent: false }, async () => {
  await expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow(/^$/);
});
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file .node-version`

```
22.18.0
```
