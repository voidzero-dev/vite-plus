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
    // Vitest v4 compatibility: preserve mock call history.
    // Remove after tests no longer rely on calls from setup or earlier tests.
    // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
    clearMocks: false,
    // Vitest v4 compatibility: keep separate Vite servers for inline projects.
    // Remove when plugins and config hooks can run once for shared projects.
    // https://vitest.dev/guide/migration/#inline-projects-share-the-vite-server-by-default
    sharedViteServer: false,
    reporters: [['json', {
      // Vitest v4 compatibility: write JSON/JUnit reports to stdout.
      // Remove after report consumers use output files; keep if stdout is required.
      // https://vitest.dev/guide/migration/#generated-reports-and-artifacts-use-the-vitest-directory
      stdout: true
    }], ['junit', {
      // Vitest v4 compatibility: write JSON/JUnit reports to stdout.
      // Remove after report consumers use output files; keep if stdout is required.
      // https://vitest.dev/guide/migration/#generated-reports-and-artifacts-use-the-vitest-directory
      stdout: true
    }]],
    projects: [{
      // Vitest v4 compatibility: keep this inline project independent of the root config.
      // Remove to inherit root options, including plugins and setup files.
      // https://vitest.dev/guide/migration/#inline-projects-inherit-the-root-config-by-default
      extends: false,
      test: {
        // Vitest v4 compatibility: preserve mock call history.
        // Remove after tests no longer rely on calls from setup or earlier tests.
        // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
        clearMocks: false,
        name: 'unit' } }, { extends: true, test: { name: 'inherited' } }],
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

## `vpt stat-file .vite-plus/migrations.json --assert missing`

```
.vite-plus/migrations.json: missing
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt stat-file .vite-plus/migrations.json --assert missing`

```
.vite-plus/migrations.json: missing
```
