# in_source_test_scopes

Migrate globals in guarded includeSource files and normal tests, preserve excluded and unguarded sources, and execute the result with Vitest 5.

## `vpt rm example.test.ts`


## `vpt write-file vite.config.ts 'export default { test: { globals: true, includeSource: ['\''src/*.js'\''], exclude: ['\''src/excluded.js'\''] } };
'`


## `vpt write-file src/add.js 'export const add = (a, b) => a + b;
if (import.meta.vitest) {
  test.sequential('\''in-source'\'', () => {
    expect(add(2, 3)).toBe(5);
    expect(() => { throw new Error('\'''\''); }).toThrow('\'''\'');
  });
}
'`


## `vpt write-file control.test.js 'test.sequential('\''normal control'\'', () => { expect(() => { throw new Error('\'''\''); }).toThrow('\'''\''); });
'`


## `vpt write-file src/excluded.js 'if (import.meta.vitest) { test.sequential('\''excluded'\'', () => { expect(() => { throw new Error('\''boom'\''); }).toThrow('\'''\''); }); }
'`


## `vpt write-file src/no-guard.js 'test.sequential('\''another runner'\'', () => { expect(() => { throw new Error('\''boom'\''); }).toThrow('\'''\''); });
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
```

## `vpt print-file src/add.js control.test.js src/excluded.js src/no-guard.js`

```
export const add = (a, b) => a + b;
if (import.meta.vitest) {
  test('in-source', { concurrent: false }, () => {
    expect(add(2, 3)).toBe(5);
    expect(() => { throw new Error(''); }).toThrow(/^$/);
  });
}
test('normal control', { concurrent: false }, () => { expect(() => { throw new Error(''); }).toThrow(/^$/); });
if (import.meta.vitest) { test.sequential('excluded', () => { expect(() => { throw new Error('boom'); }).toThrow(''); }); }
test.sequential('another runner', () => { expect(() => { throw new Error('boom'); }).toThrow(''); });
```

## `node verify-in-source.mjs`

```
Vitest 5 passed the in-source test and normal control; production import preserved
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
