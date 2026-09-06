# upstream_real_timers

Known upstream Vitest 5.0.0 release blocker, also reproduced without Vite+: Preview locator clicks call advanceTimers with real timers. Remove this expected-failure assertion when upstream is fixed.

## `node verify.mjs --real-timers`

```
Browser runner started at http://localhost:<port>/__vitest_test__/?sessionId=<uuid>

Release blocker reproduced: upstream Vitest 5.0.0 Preview locator clicks fail with real timers
```
