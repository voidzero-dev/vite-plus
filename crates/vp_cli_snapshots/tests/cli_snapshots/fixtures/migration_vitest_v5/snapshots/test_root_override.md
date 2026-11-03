# test_root_override

test.root overrides the Vite root. Migrate only that test and setup scope, leaving the similarly named unrelated suite unchanged.

## `vpt write-file vite.config.ts 'export default { root: '\''./app'\'', test: { root: '\''./unit'\'', globals: true, setupFiles: '\''./setup.js'\'' } };
'`


## `vpt write-file unit/setup.js 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file unit/right.test.js 'test.sequential('\''works'\'', () => {});
'`


## `vpt write-file app/unit/wrong.test.js 'test('\''other runner'\'', () => { expect(() => { throw new Error('\''boom'\''); }).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file unit/right.test.js`

```
test('works', { concurrent: false }, () => {});
```

## `vpt print-file unit/setup.js`

```
beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });
```

## `vpt print-file app/unit/wrong.test.js`

```
test('other runner', () => { expect(() => { throw new Error('boom'); }).toThrow(''); });
```

## `node verify-review.mjs --scopes --root`

```
Vitest 5 passed the migrated globals-only test and setup hook in the selected scope
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
