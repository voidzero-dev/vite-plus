# test_discovery_dir

Match includes and excludes under test.dir, keep setup resolution relative to root, and preserve files outside the discovery directory.

## `vpt write-file vite.config.ts 'export default { root: '\''./app'\'', test: { globals: true, dir: '\''./unit'\'', include: ['\''*.test.js'\''], exclude: ['\''excluded.test.js'\''], setupFiles: '\''../setup.js'\'' } };
'`


## `vpt write-file setup.js 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file unit/right.test.js 'test.sequential('\''works'\'', () => {});
'`


## `vpt write-file wrong.test.js 'test('\''other runner'\'', () => { expect(() => { throw Error('\''boom'\''); }).toThrow('\'''\''); });
'`


## `vpt write-file unit/excluded.test.js 'test.sequential('\''excluded'\'', () => {});
'`


## `vpt write-file app/unit/wrong.test.js 'test.sequential('\''wrong root'\'', () => {});
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file unit/right.test.js setup.js wrong.test.js unit/excluded.test.js app/unit/wrong.test.js`

```
test('works', { concurrent: false }, () => {});
beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });
test('other runner', () => { expect(() => { throw Error('boom'); }).toThrow(''); });
test.sequential('excluded', () => {});
test.sequential('wrong root', () => {});
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
