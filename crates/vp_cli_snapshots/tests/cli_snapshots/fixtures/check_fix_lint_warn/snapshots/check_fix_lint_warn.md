# check_fix_lint_warn

## `vp check --fix`

```
warn: Lint warnings found
⚠ eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:2:3]
 1 │ function hello() {
 2 │   console.log("hello");
   ·   ───────────
 3 │ }
   ╰────
  help: Delete this console statement.

Found 0 errors and 1 warning in 2 files (<duration>, <n> threads)
pass: Formatting completed for checked files (<duration>)
```

## `vp check`

```
pass: All 3 files are correctly formatted (<duration>, <n> threads)
warn: Lint warnings found
⚠ eslint(no-console): Unexpected console statement.
   ╭─[src/index.js:2:3]
 1 │ function hello() {
 2 │   console.log("hello");
   ·   ───────────
 3 │ }
   ╰────
  help: Delete this console statement.

Found 0 errors and 1 warning in 2 files (<duration>, <n> threads)
```
