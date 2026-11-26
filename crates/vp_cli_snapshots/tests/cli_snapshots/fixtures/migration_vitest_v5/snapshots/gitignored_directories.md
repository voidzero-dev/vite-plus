# gitignored_directories

Skip gitignored directories and invalid .gitignore lines without failing migration.

## `vpt write-file .gitignore '{a,b
static/
'`


## `vpt write-file static/bundle.test.ts 'import { test } from '\''vitest'\'';
test.sequential('\''ignored'\'', () => {});
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file example.test.ts static/bundle.test.ts .gitignore`

```
import { expect, test } from 'vite-plus/test';

test('compatibility', { concurrent: false }, async () => {
  await expect(Promise.resolve(42)).resolves.toBe(42);
  expect(() => { throw new Error(''); }).toThrow(/^$/);
});
import { test } from 'vitest';
test.sequential('ignored', () => {});
{a,b
static/
.vitest/
```
