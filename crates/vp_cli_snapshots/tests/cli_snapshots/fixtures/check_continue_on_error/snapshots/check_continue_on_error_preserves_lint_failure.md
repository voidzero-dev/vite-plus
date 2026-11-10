# check_continue_on_error_preserves_lint_failure

## `vpt write-file src/index.ts 'export const value: string = 42;
eval("value");
'`

```
```

## `vp check --continue-on-error src/index.ts`

formatting passes but lint and type failures still produce a nonzero exit

**Exit code:** 1

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
error: Lint or type issues found
× eslint(no-eval): eval can be harmful.
   ╭─[src/index.ts:2:1]
 1 │ export const value: string = 42;
 2 │ eval("value");
   · ────
   ╰────
  help: Avoid eval(). For JSON parsing use JSON.parse(); for dynamic property access use bracket notation (obj[key]); for other cases refactor to avoid evaluating strings as code.

  × typescript(TS2322): Type 'number' is not assignable to type 'string'.
   ╭─[src/index.ts:1:14]
 1 │ export const value: string = 42;
   ·              ─────
 2 │ eval("value");
   ╰────

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
```
