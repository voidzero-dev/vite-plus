# setup_extension_precedence

Prefer setup.mjs over setup.js, and leave the inactive JavaScript file unchanged.

## `vpt write-file vite.config.ts 'export default { test: { globals: true, setupFiles: ['\''./setup'\''], include: ['\''unit.test.js'\''] } };
'`


## `vpt write-file setup.mjs 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file setup.js 'expect(() => { throw Error('\''boom'\''); }).toThrow('\'''\'');
'`


## `vpt write-file unit.test.js 'test('\''works'\'', () => {});
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file setup.mjs setup.js`

```
beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });
expect(() => { throw Error('boom'); }).toThrow('');
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
