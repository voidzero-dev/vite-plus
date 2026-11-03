# v5_without_compatibility_reviews

Migrate an original Vitest v5 project without v4 default edits or behavior reviews.

## `vpt json-edit package.json devDependencies.vitest 5.0.1`


## `vpt write-file example.test.ts 'import { expect, it, test } from '\''vitest'\'';
it.each(['\''system'\'', '\''light'\'', '\''dark'\''])('\''accepts %s'\'', value => { expect(value).toBe(value); });
test('\''poll'\'', async () => { await expect.poll(() => 42).toBe(42); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file vite.config.ts example.test.ts package.json`

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  test: {
    reporters: ['json', ['junit', {}]],
    projects: [{ test: { name: 'unit' } }, { extends: true, test: { name: 'inherited' } }],
  },
});
import { expect, it, test } from 'vite-plus/test';
it.each(['system', 'light', 'dark'])('accepts %s', value => { expect(value).toBe(value); });
test('poll', async () => { await expect.poll(() => 42).toBe(42); });
{
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "name": "migration-vitest-v5",
  "packageManager": "pnpm@11.24.0",
  "private": true,
  "scripts": {
    "test": "vp test list"
  },
  "type": "module"
}
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
