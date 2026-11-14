# lint_vite_plus_imports_overrides

Existing lint.overrides can disable only the import rule in templates and examples, with no migrate configuration. Other lint rules remain active.

## `vp lint --threads=1 --fix app.test.ts templates/starter.test.ts examples/vite.config.ts`

**Exit code:** 1

```

  × eslint(no-debugger): `debugger` statement is not allowed
   ╭─[templates/starter.test.ts:2:1]
 1 │ import { test } from 'vitest';
 2 │ debugger;
   · ─────────
 3 │ test('template', () => {});
   ╰────
  help: Remove the debugger statement

Found 0 warnings and 1 error.
Finished in <duration> on 3 files with <n> rules using <n> threads.
```

## `vpt print-file app.test.ts`

```
import { test } from 'vite-plus/test';
test('application', () => {});
```

## `vpt print-file templates/starter.test.ts`

```
import { test } from 'vitest';
debugger;
test('template', () => {});
```

## `vpt print-file examples/vite.config.ts`

```
import { defineConfig } from 'vite';
export default defineConfig({});
```
