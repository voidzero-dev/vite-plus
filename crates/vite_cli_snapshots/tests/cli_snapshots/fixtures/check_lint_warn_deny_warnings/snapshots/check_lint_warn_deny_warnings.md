# check_lint_warn_deny_warnings

## `vp check`

**Exit code:** 1

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
