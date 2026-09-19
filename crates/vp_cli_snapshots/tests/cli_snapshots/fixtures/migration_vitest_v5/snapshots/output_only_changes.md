# output_only_changes

Keep parameterized titles and reporter options without cosmetic compatibility edits or review prompts.

## `vpt write-file example.test.ts 'import { expect, it } from '\''vitest'\'';
it.each(['\''system'\'', '\''light'\'', '\''dark'\''])('\''accepts %s'\'', value => { expect(value).toBe(value); });
'`


## `vpt write-file vite.config.ts 'export default { test: { reporters: ['\''json'\'', ['\''junit'\'', {}], ['\''html'\'', { outputFile: '\''reports/index.html'\'' }]] } };
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file vite.config.ts example.test.ts`

```
export default {
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  test: {
      // Vitest v4 compatibility: preserve mock call history.
      // Remove after tests no longer rely on calls from setup or earlier tests.
      // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
      clearMocks: false,
      reporters: ['json', ['junit', {}], ['html', { outputFile: 'reports/index.html' }]] }
}
import { expect, it } from 'vite-plus/test';
it.each(['system', 'light', 'dark'])('accepts %s', value => { expect(value).toBe(value); });
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
