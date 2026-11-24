# github

## `vp check --format=github clean.js`

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vp check --format=github error.js`

**Exit code:** 1

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
::error file=error.js,line=1,endLine=1,col=1,endColumn=5,title=eslint(no-eval)::error.js:1:1: eval can be harmful.

Found 0 warnings and 1 error.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
