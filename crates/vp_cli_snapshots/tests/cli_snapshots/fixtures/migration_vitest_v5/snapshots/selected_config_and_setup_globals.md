# selected_config_and_setup_globals

An inactive Vite config must not suppress Vitest globals. Migrate the selected test and its globals-only setup hook.

## `vpt write-file vite.config.ts 'export default {};
'`


## `vpt write-file vitest.config.mjs 'export default { test: { globals: true, include: ['\''unit/*.test.js'\''], setupFiles: '\''./setup.js'\'' } };
'`


## `vpt write-file setup.js 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
'`


## `vpt write-file unit/right.test.js 'test.sequential('\''works'\'', () => { expect(() => { throw new Error('\'''\''); }).toThrow('\'''\''); });
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
test('works', { concurrent: false }, () => { expect(() => { throw new Error(''); }).toThrow(/^$/); });
```

## `vpt print-file setup.js`

```
beforeEach(async () => { await expect(Promise.resolve(1)).resolves.toBe(1); });
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
