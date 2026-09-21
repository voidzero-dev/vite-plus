# workspace_test_api_identity

Workspace test imports must use the active Vitest runner, even when pnpm installs separate root and child peer instances.

## `vp install --ignore-scripts`


## `node verify.mjs`

```
Root and child have distinct Vite+ and Vitest peer instances
single: collection, hooks, assertions, and mocks passed
shared: collection, hooks, assertions, and mocks passed
separate: collection, hooks, assertions, and mocks passed
```

## `vp test run`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓ packages/child/identity.test.js (1 test) <duration>
   ✓ workspace test API (1)
     ✓ shares collection, assertions, and mocks with the active runner <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `IDENTITY_MODE=shared vp test run`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓  child  packages/child/identity.test.js (1 test) <duration>
   ✓ workspace test API (1)
     ✓ shares collection, assertions, and mocks with the active runner <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `IDENTITY_MODE=separate vp test run`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓  child  identity.test.js (1 test) <duration>
   ✓ workspace test API (1)
     ✓ shares collection, assertions, and mocks with the active runner <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```

## `cd packages/child && vp test run`

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>/packages/child

 ✓  child  identity.test.js (1 test) <duration>
   ✓ workspace test API (1)
     ✓ shares collection, assertions, and mocks with the active runner <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (<timing>)
```
