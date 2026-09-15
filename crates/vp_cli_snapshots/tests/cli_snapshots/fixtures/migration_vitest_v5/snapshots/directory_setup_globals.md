# directory_setup_globals

An extensionless setup directory resolves its index file and migrates its globals-only hook.

## `vpt write-file vite.config.ts 'export default { test: { globals: true, setupFiles: ['\''./setup'\''], include: ['\''unit.test.js'\''] } };
'`


## `vpt write-file setup/index.js 'beforeEach(() => { expect(Promise.resolve(1)).resolves.toBe(1); });
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

## `vpt print-file setup/index.js`

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
