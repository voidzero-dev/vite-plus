# check_fix_no_lint_typecheck_fail

## `vp check --fix --no-lint`

**Exit code:** 1

```
error: Type errors found
× typescript(TS2322): Type 'string' is not assignable to type 'number'.
   ╭─[src/index.ts:1:7]
 1 │ const value: number = "not a number";
   ·       ─────
 2 │ export { value };
   ╰────

Found 1 error and 0 warnings in 2 files (<duration>, <n> threads)
pass: Formatting completed for checked files (<duration>)
```
