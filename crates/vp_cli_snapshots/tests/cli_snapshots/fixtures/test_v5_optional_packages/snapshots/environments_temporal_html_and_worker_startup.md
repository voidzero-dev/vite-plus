# environments_temporal_html_and_worker_startup

## `npm install --ignore-scripts --no-audit --no-fund`


## `vp test run --reporter=default --reporter=html --maxWorkers=1`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓  custom  custom.test.js (1 test) <duration>
 ✓  happy-dom  happy-dom.test.js (1 test) <duration>
 ✓  jsdom  jsdom.test.js (1 test) <duration>
 ✓  temporal  temporal.test.js (3 tests) <duration>

 Test Files  4 passed (4)
      Tests  6 passed (6)
   Start at  <time>
   Duration  <duration> (<timing>)

 HTML  Report is generated
       You can run npx vite preview --outDir .vitest to see the test results.
```

## `vpt stat-file .vitest/index.html --assert file`

```
.vitest/index.html: file
```

## `node verify-ui.mjs`

```
UI rejects a bare URL, accepts its token, and serves the clean URL with an authenticated cookie
```

## `node verify-worker.mjs`

A failed worker must produce a controlled diagnostic and a nonzero exit status.

**Exit code:** 1

```
Vitest caught 1 unhandled error during the test run.
Worker exited unexpectedly with exit code 9 during starting state
```
