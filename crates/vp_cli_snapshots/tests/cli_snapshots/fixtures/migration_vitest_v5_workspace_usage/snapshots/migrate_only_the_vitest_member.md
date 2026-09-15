# migrate_only_the_vitest_member

A Vitest package-name string in the root plugin must not block migration of its Vitest v4 example. Repeated migration does not create a state file.

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied, 1 file had imports rewritten
```

## `vpt print-file src/index.ts examples/vite-8/unit.test.js`

```
// Tool names are not imports or evidence that this package runs Vitest.
export const NON_RUNTIME_PKGS = ['vite', 'vitest'];
import { expect, test } from 'vite-plus/test';

test('works', { concurrent: false }, () => {
  expect(() => { throw new Error(''); }).toThrow(/^$/);
});
```

## `vpt stat-file .vite-plus/migrations.json --assert missing`

```
.vite-plus/migrations.json: missing
```

## `cd examples/vite-8 && vp test run`

```
VITE+ - The Unified Toolchain for the Web

note: You are running `vp test` as a Vite+ built-in command. If you meant to run the test npm script, use `vpr test` instead.

 RUN  <version> <workspace>/examples/vite-8

 ✓ unit.test.js (1 test) <duration>
   ✓ works <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `vp migrate --no-interactive --no-hooks --no-agent --no-editor`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt stat-file .vite-plus/migrations.json --assert missing`

```
.vite-plus/migrations.json: missing
```
