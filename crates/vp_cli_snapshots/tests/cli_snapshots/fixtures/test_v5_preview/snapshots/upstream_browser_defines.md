# upstream_browser_defines

Regression for upstream #11198: Preview must preserve string and boolean define values. Keep this test after removing the temporary Vitest 5.0.0 backport.

## `node verify.mjs --browser-defines`

```
Browser runner started at http://localhost:<port>/__vitest_test__/?sessionId=<uuid>

Preview: browser string and boolean define values passed
```
