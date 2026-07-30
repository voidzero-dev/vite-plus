# vitest_browser_mode

## `vp run test`

```
$ vp test

 RUN  <version> <workspace>

Failed to resolve dependency: vitest > expect-type, present in client 'optimizeDeps.include'
Failed to resolve dependency: vitest > @vitest/snapshot > magic-string, present in client 'optimizeDeps.include'
Failed to resolve dependency: vitest > @vitest/expect > chai, present in client 'optimizeDeps.include'
<time> [vite] (client) warning:
<repo>/packages/core/dist/vite/node/module-runner.js
1007 |          }
1008 |          runExternalModule(filepath) {
1009 |                  return globalThis["__vitest_browser_runner__"].wrapDynamicImport(() => import(filepath));
     |                                                                                  ^^^^^^^^
1010 |          }
1011 |  };
The above dynamic import cannot be analyzed by Vite.
See https://vite.dev/guide/features#dynamic-import for supported dynamic import formats. If this is intended to be left as-is, you can use the /* @vite-ignore */ comment inside the import() call to suppress this warning.

  Plugin: vite:import-analysis
  File: <repo>/packages/core/dist/vite/node/module-runner.js
 ✓  chromium  src/foo.test.js (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)
```

## `vpt write-file src/foo.js 'export default '\''foo'\'';
//comment
'`


## `vp run test`

```
$ vp test ○ cache miss: 'src/foo.js' modified, executing

 RUN  <version> <workspace>

<time> [vite] (client) warning:
<repo>/packages/core/dist/vite/node/module-runner.js
1007 |          }
1008 |          runExternalModule(filepath) {
1009 |                  return globalThis["__vitest_browser_runner__"].wrapDynamicImport(() => import(filepath));
     |                                                                                  ^^^^^^^^
1010 |          }
1011 |  };
The above dynamic import cannot be analyzed by Vite.
See https://vite.dev/guide/features#dynamic-import for supported dynamic import formats. If this is intended to be left as-is, you can use the /* @vite-ignore */ comment inside the import() call to suppress this warning.

  Plugin: vite:import-analysis
  File: <repo>/packages/core/dist/vite/node/module-runner.js
 ✓  chromium  src/foo.test.js (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)
```

## `vpt write-file src/bar.js 'export default '\''bar'\'';
//comment
'`


## `vp run test`

```
$ vp test ◉ cache hit, replaying

 RUN  <version> <workspace>

<time> [vite] (client) warning:
<repo>/packages/core/dist/vite/node/module-runner.js
1007 |          }
1008 |          runExternalModule(filepath) {
1009 |                  return globalThis["__vitest_browser_runner__"].wrapDynamicImport(() => import(filepath));
     |                                                                                  ^^^^^^^^
1010 |          }
1011 |  };
The above dynamic import cannot be analyzed by Vite.
See https://vite.dev/guide/features#dynamic-import for supported dynamic import formats. If this is intended to be left as-is, you can use the /* @vite-ignore */ comment inside the import() call to suppress this warning.

  Plugin: vite:import-analysis
  File: <repo>/packages/core/dist/vite/node/module-runner.js
 ✓  chromium  src/foo.test.js (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)

---
vp run: cache hit, <duration> saved.
```
