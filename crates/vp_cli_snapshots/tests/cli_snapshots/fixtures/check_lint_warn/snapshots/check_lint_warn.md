# check_lint_warn

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

## `vp check --quiet`

warning diagnostics are suppressed

```
pass: All 3 files are correctly formatted (<duration>, <n> threads)

Found 0 errors and 1 warning in 2 files (<duration>, <n> threads)
```

## `vp lint --quiet`

standalone lint has the same warning suppression semantics

```

Found 1 warning and 0 errors.
Finished in <duration> on 2 files with <n> rules using <n> threads.
```
