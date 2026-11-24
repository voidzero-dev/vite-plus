# default

## `vp check clean.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `vp check --format=default clean.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `GITHUB_ACTIONS=true vp check --format=default warning.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
warn: Lint warnings found
⚠ eslint(no-console): Unexpected console statement.
   ╭─[warning.js:1:1]
 1 │ console.log("warning");
   · ───────────
   ╰────
  help: Delete this console statement.

Found 0 errors and 1 warning in 1 file (<duration>, <n> threads)
```
