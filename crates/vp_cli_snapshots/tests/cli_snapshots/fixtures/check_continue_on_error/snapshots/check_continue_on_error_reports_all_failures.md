# check_continue_on_error_reports_all_failures

## `vp check src/index.ts`

default behavior stops at formatting failures

**Exit code:** 1

```
error: Formatting issues found
src/index.ts (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
```

## `vp check --continue-on-error src/index.ts`

format, lint, and type diagnostics are all reported

**Exit code:** 1

```
error: Formatting issues found
src/index.ts (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
error: Lint or type issues found
× eslint(no-eval): eval can be harmful.
   ╭─[src/index.ts:2:1]
 1 │ export const value:string=42
 2 │ eval("value")
   · ────
   ╰────
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[src/index.ts:1:14]
 1 │ export const value:string=42
   ·              ─────
 2 │ eval("value")
   ╰────

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
```

## `vp check --continue-on-error --no-fmt src/index.ts`

formatting can still be skipped

**Exit code:** 1

```
error: Lint or type issues found
× eslint(no-eval): eval can be harmful.
   ╭─[src/index.ts:2:1]
 1 │ export const value:string=42
 2 │ eval("value")
   · ────
   ╰────
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[src/index.ts:1:14]
 1 │ export const value:string=42
   ·              ─────
 2 │ eval("value")
   ╰────

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
```

## `vp check --continue-on-error --no-lint src/index.ts`

skip lint rules while retaining format and type errors

**Exit code:** 1

```
error: Formatting issues found
src/index.ts (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
error: Type errors found
× typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[src/index.ts:1:14]
 1 │ export const value:string=42
   ·              ─────
 2 │ eval("value")
   ╰────

Found 1 error and 0 warnings in 1 file (<duration>, <n> threads)
```
