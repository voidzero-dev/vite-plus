# packed_browser_defines

The define regression must also pass with published-package layouts and install scripts disabled.

## `node prepare-packed.mjs`


## `vp install --ignore-scripts`


## `node verify.mjs --packed`

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
VITE+ - The Unified Toolchain for the Web

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
