# cli_discovery_dir

A literal --dir overrides config discovery without changing setup-file ownership.

## `vpt json-edit package.json scripts.test 'vitest run --dir ./unit'`


## `vpt write-file vite.config.ts 'export default { test: { globals: true, dir: '\''./ignored'\'', include: ['\''*.test.js'\''], setupFiles: '\''./setup.js'\'' } };
'`


## `vpt write-file setup.js 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file unit/right.test.js 'test.sequential('\''works'\'', () => {});
'`


## `vpt write-file wrong.test.js 'expect(() => { throw Error('\''boom'\''); }).toThrow('\'''\'');
'`


## `vpt write-file ignored/wrong.test.js 'test.sequential('\''ignored'\'', () => {});
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file unit/right.test.js setup.js wrong.test.js ignored/wrong.test.js`

```
test('works', { concurrent: false }, () => {});
beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });
expect(() => { throw Error('boom'); }).toThrow('');
test.sequential('ignored', () => {});
```

## `node verify-review.mjs --scopes --dir ./unit`

```
Vitest 5 passed the migrated globals-only test and setup hook in the selected scope
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
