# vue_style_global_scopes

Resolve inherited globals through callbacks, imported config defaults, conditional projects, literal command lists, and root-relative setup paths. Preserve unrelated suites.

## `vpt json-edit package.json scripts.test 'node build.js && MODE=unit vitest run --project unit*'`


## `vpt write-file vite.config.ts 'import { configDefaults, defineConfig } from '\''vite-plus'\'';
export default defineConfig({ test: { globals: true, setupFiles: '\''scripts/setup.ts'\'', onConsoleLog(log) { return '\!'log.includes('\''quiet'\''); }, projects: [
  { extends: true, test: { name: '\''unit'\'', include: ['\''unit/*.test.ts'\''], exclude: [...configDefaults.exclude] } },
  ...(process.env.EXTRA ? [{ extends: true, test: { name: '\''extra'\'', include: ['\''extra/*.test.ts'\''] } }] : []),
] } });
'`


## `vpt write-file scripts/setup.ts 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file unit/right.test.ts 'test.sequential('\''works'\'', () => { expect(1).toBe(1); });
'`


## `vpt write-file other/wrong.test.ts 'test('\''other runner'\'', () => { expect(() => { throw Error('\''boom'\''); }).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file unit/right.test.ts scripts/setup.ts other/wrong.test.ts`

```
test('works', { concurrent: false }, () => { expect(1).toBe(1); });
beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });
test('other runner', () => { expect(() => { throw Error('boom'); }).toThrow(''); });
```

## `node verify-review.mjs --scopes`

```
Vitest 5 passed the migrated globals-only test and setup hook in the selected scope
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
