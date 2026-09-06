# upstream_browser_defines

Known upstream Vitest 5.0.0 release blocker, also reproduced without Vite+: browser globals receive JSON-encoded define values. Remove this expected-failure assertion when upstream is fixed.

## `node verify.mjs --browser-defines`

```
Browser runner started at http://localhost:<port>/__vitest_test__/?sessionId=<uuid>

Release blocker reproduced: upstream Vitest 5.0.0 browser globals retain JSON-encoded define values
```
