# global_ownership_and_sequential_timeouts

Migrate only the Vitest globals and preserve numeric sequential timeouts in runtime collection. Leave the separate Jest suite unchanged.

## `vpt write-file vite.config.ts 'export default { test: { globals: true, include: ['\''unit/**/*.test.js'\''] } };
'`


## `vpt write-file jest.config.cjs 'module.exports = { testMatch: ['\''<rootDir>/integration/**/*.test.js'\''] };
'`


## `vpt write-file unit/slow.test.js 'test.sequential('\''slow test'\'', async () => {}, 15000);
it.sequential('\''slow it'\'', async () => {}, 12000);
'`


## `vpt write-file integration/other.test.js 'test('\''throws'\'', () => { expect(() => { throw new Error('\''boom'\''); }).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 file had imports rewritten
```

## `vpt print-file unit/slow.test.js`

```
test('slow test', { concurrent: false, timeout: 15000 }, async () => {});
it('slow it', { concurrent: false, timeout: 12000 }, async () => {});
```

## `vpt print-file integration/other.test.js`

```
test('throws', () => { expect(() => { throw new Error('boom'); }).toThrow(''); });
```

## `node verify-review.mjs`

```
Runtime collection preserved sequential timeouts: test=15000, it=12000
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
