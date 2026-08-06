# vitest_browser_mode

## `vp run test`

```
$ vp test

 RUN  <version> <workspace>

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

 ✓  chromium  src/foo.test.js (1 test) <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)

---
vp run: cache hit, <duration> saved.
```
