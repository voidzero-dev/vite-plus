# compatibility

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  test: {
    clearMocks: false,
    sharedViteServer: false,
    reporters: [['json', { stdout: true }], ['junit', { stdout: true }]],
    projects: [{ extends: false,  test: { clearMocks: false,  name: 'unit' } }, { extends: true, test: { name: 'inherited' } }],
  },
});
```

## `vpt print-file example.test.ts`

```
import { expect, test } from 'vite-plus/test';

test('compatibility', { concurrent: false }, async () => {
  await expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow(/^$/);
});
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

## `vpt print-file .vite-plus/migrations.json`

```
{
  "version": 1,
  "vitest5": {
    ".": {
      "sourceVersion": "4.1.11",
      "configless": false
    }
  }
}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
