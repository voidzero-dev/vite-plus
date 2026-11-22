# awaited_assertion_bindings

Do not review local assertion promises awaited or returned after type checks, including on a migration rerun.

## `vpt write-file example.test.ts 'import { expect, expectTypeOf, it } from '\''vitest'\'';
it('\''awaits an assertion'\'', async () => {
  const assertion = expect(Promise.resolve(1)).resolves.toBe(1);
  expectTypeOf(assertion).toEqualTypeOf<Promise<void>>();
  await assertion;
});
it('\''returns an assertion'\'', () => {
  const assertion = (expect(Promise.resolve(2)).resolves.toBe(2) as Promise<void>) satisfies Promise<void>;
  expectTypeOf(assertion).toEqualTypeOf<Promise<void>>();
  return assertion'\!';
});
'`


## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 files had imports rewritten
```

## `vpt print-file example.test.ts`

```
import { expect, expectTypeOf, it } from 'vite-plus/test';
it('awaits an assertion', async () => {
  const assertion = expect(Promise.resolve(1)).resolves.toBe(1);
  expectTypeOf(assertion).toEqualTypeOf<Promise<void>>();
  await assertion;
});
it('returns an assertion', () => {
  const assertion = (expect(Promise.resolve(2)).resolves.toBe(2) as Promise<void>) satisfies Promise<void>;
  expectTypeOf(assertion).toEqualTypeOf<Promise<void>>();
  return assertion!;
});
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
