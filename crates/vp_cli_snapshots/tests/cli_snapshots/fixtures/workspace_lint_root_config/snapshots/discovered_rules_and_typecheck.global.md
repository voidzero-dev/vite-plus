# discovered_rules_and_typecheck

## `vp lint index.ts`

Oxlint discovers the package lint block without the root rules or type-check options.

```
VITE+ - The Unified Toolchain for the Web

Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp check --no-fmt index.ts`

The check command keeps the root lint rules and type-check options.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Lint or type issues found
× eslint(no-console): Unexpected console statement.
   ╭─[index.ts:2:1]
 1 │ export const value: number = "not a number";
 2 │ console.log(value);
   · ───────────
   ╰────
  help: Delete this console statement.

  × typescript(TS2322): Type 'string' is not assignable to type 'number'.
   ╭─[index.ts:1:14]
 1 │ export const value: number = "not a number";
   ·              ─────
 2 │ console.log(value);
   ╰────

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
```

## `vp check --no-fmt --no-lint index.ts`

Type-check-only mode still reports the type error without lint rules.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Type errors found
× typescript(TS2322): Type 'string' is not assignable to type 'number'.
   ╭─[index.ts:1:14]
 1 │ export const value: number = "not a number";
   ·              ─────
 2 │ console.log(value);
   ╰────

Found 1 error and 0 warnings in 1 file (<duration>, <n> threads)
```

## `vp lint -c ../../vite.config.ts index.ts`

Explicit root selection enables the root rules and type checking while preserving package-relative file paths.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

  × eslint(no-console): Unexpected console statement.
   ╭─[index.ts:2:1]
 1 │ export const value: number = "not a number";
 2 │ console.log(value);
   · ───────────
   ╰────
  help: Delete this console statement.

  × typescript(TS2322): Type 'string' is not assignable to type 'number'.
   ╭─[index.ts:1:14]
 1 │ export const value: number = "not a number";
   ·              ─────
 2 │ console.log(value);
   ╰────

Found 0 warnings and 2 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
