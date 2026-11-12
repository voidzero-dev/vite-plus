# check_fix_dangerously_no_lint

## `vp check --fix-dangerously --no-lint src/index.js`

```
pass: Formatting completed for checked files (<duration>)
```

## `vpt print-file src/index.js`

```
const unused = 1;
export function greet(enabled) {
  if (enabled) return "hello";
  return "world";
}
```

## `vp check --no-fmt src/index.js`

lint was skipped, so the unused variable remains

**Exit code:** 1

```
error: Lint issues found
× eslint(no-unused-vars): Variable 'unused' is declared but never used. Unused variables should start with a '_'.
   ╭─[src/index.js:1:7]
 1 │ const unused = 1;
   ·       ───┬──
   ·          ╰── 'unused' is declared here
 2 │ export function greet(enabled) {
   ╰────
  help: Consider removing this declaration.

  × eslint(curly): Expected { after 'if' condition.
   ╭─[src/index.js:3:16]
 2 │ export function greet(enabled) {
 3 │   if (enabled) return "hello";
   ·                ───────────────
 4 │   return "world";
   ╰────
  help: Replace `return "hello";` with `{return "hello";}`.

Found 2 errors and 0 warnings in 1 file (<duration>, <n> threads)
```
