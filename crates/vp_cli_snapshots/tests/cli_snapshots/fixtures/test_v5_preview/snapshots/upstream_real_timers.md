# upstream_real_timers

Known upstream regression introduced in Vitest 4.1.1 and still present in 5.0.1, not a new v5 release blocker: Preview locator clicks advance fake timers while using real timers. Require success when upstream fixes it.

## `node verify.mjs --real-timers`

```
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Browser runner started at http://localhost:<port>/__vitest_test__/?sessionId=<uuid>

Known upstream regression since Vitest 4.1.1: Preview locator clicks fail with real timers in 5.0.1
```
