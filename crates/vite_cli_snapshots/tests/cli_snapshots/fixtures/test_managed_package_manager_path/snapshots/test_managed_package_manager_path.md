# test_managed_package_manager_path

The legacy case built a sanitized PATH (a mktemp dir holding only `node`, plus
/bin:/usr/bin) via `sh -c` to prove `vp test` exposes the managed package
manager to the test process even when no host pnpm is reachable
(src/managed-pm-path.test.ts runs `pnpm --version` and asserts the pinned
version). The runner's case environment is already hermetic: its PATH holds
only the case-owned Vite+ bins plus the system tail, so no host pnpm can
satisfy the lookup and the assertion exercises the managed path end to end. A
step-level PATH override would break `vp` resolution itself, which follows the
step PATH.

## `vp test --slowTestThreshold 10000`

no host pnpm reachable; vp test still exposes the managed pnpm

```
VITE+ - The Unified Toolchain for the Web

 RUN  <version> <workspace>

 ✓ src/managed-pm-path.test.ts (1 test) <duration>
   ✓ direct test command exposes the configured package manager on PATH <duration>

 Test Files  1 passed (1)
      Tests  1 passed (1)
   Start at  <time>
   Duration  <duration> (transform <duration>, setup <duration>, import <duration>, tests <duration>, environment <duration>)
```
