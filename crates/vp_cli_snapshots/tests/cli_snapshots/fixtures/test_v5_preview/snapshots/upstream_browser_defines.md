# upstream_browser_defines

Regression for upstream #11198, fixed in 5.0.1: Preview must preserve string and boolean define values without a Vite+ backport.

## `node verify.mjs --browser-defines`

```
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Browser runner started at http://localhost:<port>/__vitest_test__/?sessionId=<uuid>

Preview: browser string and boolean define values passed
```
