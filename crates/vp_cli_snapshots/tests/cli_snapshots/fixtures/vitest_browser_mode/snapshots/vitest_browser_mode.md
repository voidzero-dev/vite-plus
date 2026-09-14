# vitest_browser_mode

## `vp run test`

```
$ vp test
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.

 RUN  <version> <workspace>
      API started at http://localhost:<port>/

 ✓  chromium  src/foo.test.js (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `vpt write-file src/foo.js 'export default '\''foo'\'';
//comment
'`


## `vp run test`

```
$ vp test ○ cache miss: 'src/foo.js' modified, executing
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.

 RUN  <version> <workspace>
      API started at http://localhost:<port>/

 ✓  chromium  src/foo.test.js (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `vpt write-file src/bar.js 'export default '\''bar'\'';
//comment
'`


## `vp run test`

```
$ vp test ◉ cache hit, replaying
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.

 RUN  <version> <workspace>
      API started at http://localhost:<port>/

 ✓  chromium  src/foo.test.js (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)

---
vp run: cache hit, <duration> saved.
```
