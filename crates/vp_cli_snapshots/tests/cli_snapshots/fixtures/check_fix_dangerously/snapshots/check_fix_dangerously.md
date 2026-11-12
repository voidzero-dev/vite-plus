# check_fix_dangerously

## `vp check --fix`

safe fixes leave the unused variable diagnostic

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

Found 1 error and 0 warnings in 2 files (<duration>, <n> threads)
pass: Formatting completed for checked files (<duration>)
```

## `vp check --fix-dangerously`

dangerous fixes do not require --fix

```
pass: Formatting completed for checked files (<duration>)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)
```

## `vpt print-file src/index.js`

```
export function greet(enabled) {
  if (enabled) {
    return "hello";
  }
  return "world";
}
```

## `vp check`

```
pass: All 3 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 2 files (<duration>, <n> threads)
```
