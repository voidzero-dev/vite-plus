# check_backpressure_nonblocking_stdout

vp check exposes the stdout EAGAIN failure when a large diagnostic replay meets a non-blocking, backpressured pipe (#2165).

## `vpt backpressure-run --digest 6,8 -- vp check`

```
--- stdout ---
stdout: 1282 lines
pass: All 3 files are correctly formatted (<duration>, <n> threads)
! eslint(no-unused-vars): Variable 'unused000' is declared but never used. Unused variables should start with a '_'.
   ,-[src/index.js:2:9]
 1 | export function emitDiagnostics() {
 2 |   const unused000 = 0;
   :         ^^^^|^^^^
... 1268 lines elided ...
 129 |   const unused127 = 127;
     :         ^^^^|^^^^
     :             `-- 'unused127' is declared here
 130 | }
     `----
  help: Consider removing this declaration.

Found 0 errors and 128 warnings in 2 files (<duration>, <n> threads)
--- stderr ---
stderr: 1 lines
warn: Lint warnings found
```
