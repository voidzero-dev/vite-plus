# browser_defines

Regression for Vitest #11198, fixed upstream in 5.0.1. Raw configs, project inheritance, programmatic APIs, and the CLI must preserve define values without a Vite+ backport.

## `node prepare-local.mjs`


## `node verify.mjs`

```
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
raw: 1 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
helper: 1 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
shared: 2 projects; string, boolean, number, object, expression, and dotted defines passed
shared-node: 2 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
inherited: 2 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
separate: 2 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
independent: 3 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
referenced: 2 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
nested: 2 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
injected: 3 projects; string, boolean, number, object, expression, and dotted defines passed
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.
injected-hook: 2 projects; string, boolean, number, object, expression, and dotted defines passed
```

## `vp test --config vite.config.mjs`

```
Plugin "vitest:mocks:interceptor" defines Vite-specific hooks (configureServer) in a plugin returned from applyToEnvironment. These hooks will be ignored.

 RUN  <version> <workspace>
      API started at http://localhost:<port>/

 ✓  raw (chromium)  defines.test.js (3 tests) <duration>
   ✓ string defines preserve quotes inside the value <duration>
   ✓ boolean defines retain their values and truthiness <duration>
   ✓ number, object, expression, and dotted defines retain Vite semantics <duration>

 Test Files  1 passed (1)
      Tests  3 passed (3)
   Start at  <time>
   Duration  <duration> (<timing>)
```
