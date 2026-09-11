# browser_defines

Regression for Vitest #11198. Keep after removing the temporary 5.0.0 backport. Raw configs, project inheritance, programmatic APIs, and the CLI must preserve define values.

## `node prepare-local.mjs`


## `node verify.mjs`

```
raw: 1 projects; string, boolean, number, object, expression, and dotted defines passed
helper: 1 projects; string, boolean, number, object, expression, and dotted defines passed
shared: 2 projects; string, boolean, number, object, expression, and dotted defines passed
shared-node: 2 projects; string, boolean, number, object, expression, and dotted defines passed
inherited: 2 projects; string, boolean, number, object, expression, and dotted defines passed
separate: 2 projects; string, boolean, number, object, expression, and dotted defines passed
independent: 3 projects; string, boolean, number, object, expression, and dotted defines passed
referenced: 2 projects; string, boolean, number, object, expression, and dotted defines passed
nested: 2 projects; string, boolean, number, object, expression, and dotted defines passed
injected: 3 projects; string, boolean, number, object, expression, and dotted defines passed
injected-hook: 2 projects; string, boolean, number, object, expression, and dotted defines passed
```

## `vp test --config vite.config.mjs`

```

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
