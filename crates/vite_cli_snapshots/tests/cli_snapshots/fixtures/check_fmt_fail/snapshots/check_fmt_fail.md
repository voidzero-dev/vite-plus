# check_fmt_fail

## `vp check`

**Exit code:** 1

```
error: Formatting issues found
src/index.js (<duration>)

Found formatting issues in 1 file (<duration>, <n> threads). Run `vp check --fix` to fix them.
```

## `vp check --fix`

```
pass: Formatting completed for checked files (<duration>)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```

## `vp check`

should pass after fix

```
pass: All 2 files are correctly formatted (<duration>, <n> threads)
pass: Found no warnings or lint errors in 1 file (<duration>, <n> threads)
```
