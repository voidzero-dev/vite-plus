# RFC: Upgrade the Bundled Vitest to v5

- Status: Proposed
- Tracking issue: [#2405](https://github.com/voidzero-dev/vite-plus/issues/2405)
- Upstream baseline: [`v5.0.1`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.1)
- Release target: Vite+ before `1.0`

## Decision

Upgrade `vp test` and the `vite-plus/test*` API to Vitest `5.0.1` before Vite+ 1.0. Pin the official Vitest packages to this patch release.

The upgrade has four parts:

1. Update the bundled runner, browser packages, public shims, resolver, and Vite config integration as one versioned unit.
2. Add a Vitest v5 pass to `vp migrate`. It preserves v4 test execution behavior where a config option can do so, applies safe source rewrites, and reports changes that need review.
3. Raise the `vite-plus` Node.js range to the intersection of the current Vite+ range and the Vitest v5 range: `^22.18.0 || ^24.11.0 || >=26.0.0`.
4. Release the change through a Vite+ prerelease and run the compatibility matrix in this RFC before a stable release.

New projects and configless projects use the Vitest v5 defaults. Migrated v4 configs get explicit compatibility options with comments that explain how to adopt the new behavior. Projects already on v5 keep their choices.

## Context

Before this upgrade, Vite+ pinned `vitest` and the managed `@vitest/*` family to `4.1.11` in `pnpm-workspace.yaml`. The `vite-plus` package:

- resolves and starts its bundled Vitest binary from `packages/cli/src/resolve-test.ts`;
- generates `vite-plus/test*` exports from the installed Vitest export map in `packages/cli/build.ts`;
- projects browser providers and selected `@vitest/*` packages into extra compatibility paths;
- forces Vitest-family imports to the bundled package graph in `packages/cli/src/define-config.ts`;
- injects resolver, dependency-inline, and coverage-version plugins into the root and each inline test project;
- rewrites Vitest imports and aligns Vitest ecosystem packages during `vp migrate`.

These parts enforce one physical Vitest instance. That property must remain. Mock state, assertion state, browser types, coverage providers, and runner state can fail when a project loads two copies.

Vitest v5 changes the package graph and project configuration model. A version-only catalog update would create invalid exports, duplicate Vite+ plugins in inline projects, retain removed packages, and keep migration rules that emit paths which no longer exist.

## Goals

- Keep `vp test`, `vite-plus/test*`, browser mode, coverage, and config helpers on one compatible Vitest v5 graph.
- Preserve v4 test execution behavior in existing configs when a v5 option provides an exact compatibility setting.
- Give a file and an action for each change that cannot be automated safely.
- Keep useful Vite+ legacy aliases through the Vite+ 1.x line when they have an exact v5 target.
- Test Node, package manager, project, browser, coverage, reporter, and programmatic API boundaries before release.

## Non-goals

- Reimplement removed Vitest internals.
- Redesign complex benchmark comparisons and custom reporting integrations automatically.
- Migrate or report changes that affect only title formatting, report output, artifact paths, or dependency deprecation.
- Hide new Vitest behavior inside the runner after migration.
- Support Vitest v5 on Node 20 or Node 25.
- Require the community WebDriverIO provider to publish in lockstep with Vitest.

## Upstream audit

The audit covers every v5 prerelease, the final [`v5.0.0` release](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0), and the complete [Vitest v5 migration guide](https://vitest.dev/guide/migration#vitest-5). Vitest v5 introduced breaking changes in these groups:

| Release                                                                     | Breaking-change themes                                                                                                                                 |
| --------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| [`beta.1`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-beta.1) | `pretty-format` output, title formatting, browser iframe scaling, coverage glob matching, and `toThrow('')                                             |
| [`beta.2`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-beta.2) | artifact paths, removed sequential APIs, browser automocks, serialized locators, inlined expect, blob reports, and removed entry points                |
| [`beta.3`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-beta.3) | Node and Vite prerequisites, and strict `expect.poll` timeout handling                                                                                 |
| [`beta.4`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-beta.4) | top-level hoisted mocks, strict browser assertions and locators, and the benchmark rewrite                                                             |
| [`beta.5`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-beta.5) | config lookup, runner packaging, DOM globals, worker IDs, browser sessions, and per-file coverage thresholds                                           |
| [`beta.6`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-beta.6) | screenshot config, fake timers, WebDriverIO ownership, mock clearing, report defaults, UI authentication, and worker-start failure handling            |
| [`beta.7`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-beta.7) | the `resolveConfig` return value                                                                                                                       |
| [`rc.1`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-rc.1)     | project inheritance, nested projects, shared servers, test-name separators, async assertions, failure screenshots, class mocks, and assertion generics |
| [`rc.2`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-rc.2)     | no new breaking change                                                                                                                                 |
| [`rc.3`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-rc.3)     | no new breaking change; Istanbul coverage moved to the `@vitest/istanbul-lib-*` packages                                                               |
| [`rc.4`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0-rc.4)     | `vitest list` and the programmatic `collect()` API use static parsing by default                                                                       |
| [`v5.0.0`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0)        | no new breaking change after `rc.4`                                                                                                                    |

Vitest published `v5.0.0` on September 3, 2026. The comparison from `rc.4` to the final tag contains fixes, documentation, dependency updates, and performance work. It contains no additional breaking commit. The final package manifests retain the audited Node, Vite, export, browser-provider, and Istanbul dependency contracts.

The Vitest maintainers released [`v5.0.1`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.1) on September 15, 2026, with the browser `define` fix from [#11198](https://github.com/vitest-dev/vitest/pull/11198). Use the upstream fix and remove the temporary Vite+ backport. Keep the browser regression fixtures.

## Compatibility design

### 1. Runtime and dependency graph

Pin the official Vitest packages to the exact version `5.0.1`. Keep the existing coverage-provider version guard. Coverage packages remain project-installed peers and must match the bundled runner exactly.

| Package group                                                                                 | Policy                                                                                                                                                                                                                                |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vitest`                                                                                      | Exact dependency and the only runner started by `vp test`                                                                                                                                                                             |
| `@vitest/browser`, `@vitest/browser-playwright`, `@vitest/browser-preview`                    | Exact version equal to the runner                                                                                                                                                                                                     |
| `@vitest/mocker`, `@vitest/pretty-format`, `@vitest/snapshot`, `@vitest/spy`, `@vitest/utils` | Keep only when a public Vite+ shim needs the standalone package; pin exactly                                                                                                                                                          |
| `@vitest/expect`                                                                              | Remove from the bundled graph; route Vitest assertions through the root `vitest` entry so assertion state stays shared                                                                                                                |
| `@vitest/runner`                                                                              | Remove from catalogs, dependencies, shims, migration alignment, and resolver assumptions; v5 no longer publishes it with the runner                                                                                                   |
| `@vitest/ws-client`                                                                           | Do not add a Vite+ shim; retain standalone use if needed without a deprecation-only migration warning                                                                                                                                 |
| `@vitest/browser-webdriverio`                                                                 | Keep as an optional compatibility peer backed by the [community repository](https://github.com/vitest-community/vitest-webdriverio); use a compatible range and test a selected version instead of forcing the runner's exact version |
| `@vitest/coverage-v8`, `@vitest/coverage-istanbul`, `@vitest/ui`                              | Project-installed optional packages; require the exact runner version                                                                                                                                                                 |
| `@vitest/web-worker`                                                                          | Project-installed optional package; keep its exact Vitest peer aligned without adding a Vite+ shim                                                                                                                                    |

Replace the resolver's broad `@vitest/*` rule with an explicit supported-package set. In particular, do not redirect a user's standalone `@vitest/expect` to a Vite+ copy. Keep bundle-first resolution for the root `vitest` package, its public subpaths, the official browser packages, and other packages that share runner state. Keep the project fallback for optional peers.

Provide Vitest's required `vite` peer through a published dependency alias to the matching `@voidzero-dev/vite-plus-core` version. Test a Yarn install with no project `vite` or `vitest` dependency; Yarn does not install a missing peer for the bundled runner.

For npm projects that declare `vite` without overrides, use an exact core-version alias matching `vite-plus`. npm can replace a tagged alias such as `npm:@voidzero-dev/vite-plus-core@latest` with upstream Vite while resolving Vitest's peer. Test both shared core identity for the exact alias and the CLI's rejection of the replaced alias. Keep the `vp migrate` repair path, which writes an exact alias and package-manager overrides.

The repository vendors Vite and Rolldown workspaces. Keep the root Vitest v5 pin when merging their catalogs during `sync-remote`, and align Vite's direct `@vitest/*` dependencies with that pin so CI can install with `--frozen-lockfile`. Leave upstream test-suite migrations to the Vite and Rolldown maintainers. Do not add their suites to CI or their test fixtures to the root workspace for this upgrade. Validate the Vite+ integration through its unit tests and CLI/browser regression fixtures.

Use the shared TypeScript helper at `packages/tools/src/vendored-vitest.ts` for both `sync-remote` and CI checkout alignment. CI runs it with `npx tsx` before Node setup and dependency installation; the runner's preinstalled Node may not support direct TypeScript execution. Repeated synchronization must leave the workspace manifest unchanged.

Vitest `5.0.0` uses packages from the `vitest-dev/istanbuljs` repository for Istanbul internals: `@vitest/istanbul-lib-coverage`, `@vitest/istanbul-lib-instrument`, `@vitest/istanbul-lib-report`, and `@vitest/istanbul-lib-source-maps`. Keep those packages on the project-installed `@vitest/coverage-istanbul` dependency edge. The resolver allowlist must not redirect them to a missing Vite+ copy.

### 2. Node.js and Vite prerequisites

Vitest v5 supports Node `^22.12.0 || ^24.0.0 || >=26.0.0` and Vite `^6.4.0 || ^7.0.0 || ^8.0.0`. The bundled Vite satisfies the Vite range.

Set the published `vite-plus` engine to `^22.18.0 || ^24.11.0 || >=26.0.0`. This removes Node 20 and Node 25 from the CLI package. Do not raise the standalone core package's engine only because of Vitest.

Use the selected project runtime for `vp test` without a separate Node-version check at command startup. Keep the requirements in package metadata and migration checks. Users who select an unsupported Node version may get an upstream error instead of a custom diagnostic.

Limit the migration preflight's Node compatibility checks to `.node-version`, `.nvmrc`, and the `engines.node`, `devEngines.runtime`, and `volta.node` declarations in `package.json`. Exclude Node versions in CI workflows, containers, and other files.

Automatically upgrade incompatible runtime pins before installing dependencies. Select the nearest supported minimum at or above the old version: `20.19.0` becomes `22.18.0`, `24.10.0` becomes `24.11.0`, and `25.9.0` becomes `26.0.0`. Preserve supported pins and forward-moving aliases such as `lts/*` and `latest`. Report selectors that cannot be resolved for review.

Keep public `engines.node` contracts unchanged. Do not report an engine range with a supported minimum, such as `>=22.19.0`, or a whole supported major, such as `24.x`. A library's public engine contract and its concrete test runtime serve different purposes.

### 3. Public `vite-plus/test*` exports

Continue to generate the main test surface from the v5 `vitest` export map, then add a reviewed compatibility map. Snapshot the final export keys in a test so an upstream export change cannot alter the Vite+ package by accident.

Keep these aliases through the Vite+ 1.x line because each has an exact public target:

| Vite+ compatibility path      | v5 target        |
| ----------------------------- | ---------------- |
| `vite-plus/test/coverage`     | `vitest/node`    |
| `vite-plus/test/reporters`    | `vitest/node`    |
| `vite-plus/test/environments` | `vitest/runtime` |
| `vite-plus/test/snapshot`     | `vitest/runtime` |
| `vite-plus/test/mocker`       | `@vitest/mocker` |

Do not create partial shims for `vite-plus/test/runners`, `vite-plus/test/suite`, `vite-plus/test/plugins/runner`, `vite-plus/test/plugins/expect`, or `vite-plus/test/internal/module-runner`. Their old symbols do not have a complete one-to-one v5 implementation with the required shared state. The migration reports them and directs users to `expect`, `TestRunner`, and its static methods from `vite-plus/test` where possible.

Keep the Playwright and Preview provider aliases. Keep the WebDriverIO aliases only when the community peer is installed and compatible. Update `packages/cli/BUNDLING.md` to state which paths mirror upstream and which paths are Vite+ compatibility contracts.

Route the existing browser-context runtime aliases to `vitest/browser`. This includes provider `/context` paths, `vite-plus/test/browser/context`, `vite-plus/test/context`, and `vite-plus/test/plugins/browser-context`. Keep their upstream type declarations and augmentations. In v5, the `@vitest/browser/context` JavaScript export is an error stub; the browser plugin resolves only `vitest/browser` as its virtual context module.

Use the final community WebDriverIO provider `5.0.0` as the compatibility test baseline with Vitest `5.0.1`. Keep the optional peer range `^5.0.0-beta.5 || >=5.0.0` from the final Vitest manifest. Test future community releases before changing the baseline; the provider follows its own release schedule.

### 4. Config integration and project inheritance

Vitest v4 inline projects did not inherit root plugins. The v4 Vite+ wrapper therefore prepended its three plugins to the root and every inline project. Vitest v5 sets `extends: true` by default and merges plugin arrays. Keeping the old injection behavior would register each Vite+ plugin twice.

Change the injection algorithm as follows:

1. Inject the resolver, dependency-inline, and coverage-version plugins into the root config.
2. For an inline project that inherits the declaring config, rely on the inherited plugins.
3. For `extends: false` or an explicit external base config, inject the plugins into that project.
4. Apply the same rule after resolving function and promise project entries.
5. Make each Vite+ plugin idempotent by name as a defense against user merges and nested referenced configs.
6. Keep the coverage guard keyed by the shared Vitest instance because `configureVitest` can run for several projects.

Referenced config files and directories still resolve their own Vite config. Their `defineConfig` or `defineProject` call injects the Vite+ plugins. Add fixtures for raw config files that do not use the Vite+ helpers and give them the existing soft fallback behavior.

Keep Vitest's directory-local config lookup. Do not scan parent directories or print a startup warning. From a subdirectory, users can select the parent config with `vp test --config ../vite.config.ts --dir .`; document this command in the migration guide.

Limit matcher-dependency detection to Vitest servers. Cache installed and missing packages per project root during each config load, then clear the cache on config reload. Cache the bundled Vitest export metadata across browser entry points and project configs.

## Migration design

Add a versioned Vitest v5 migration pass. Run its preflight before package changes. If blockers exist, print them and stop before edits or dependency updates. Otherwise, perform safe edits and print unresolved review items once in the final summary, grouped by file. Include a documentation link with each Vitest v5 diagnostic.

Determine the source runner version before updating dependencies, including catalog and installed Vite+ dependencies. Keep that version in memory through the current invocation. Do not read or write a migration state file. On later runs, resolve the runner version again and retain v5 choices without applying v4 defaults, including in configless projects.

Require installed or lockfile evidence when a dependency range spans both v4 and v5. Accept manifest ranges that identify one supported source major without requiring an installed runner. Retain deferred source-review findings through the current run's final report without reapplying its v4 edits. Later runs report issues detectable from current files; users must resolve or save v4-specific reviews before discarding the original dependency evidence. Ignore state files from earlier previews and leave their removal to the user.

Apply the Node policy above before installers run. Restrict migration edits and diagnostics to execution compatibility. Leave title formatting, regular reporter options, generated output paths, and dependency deprecation unchanged and unreported. Removed benchmark APIs and flags still need migration so commands can run.

### Behavior-preserving config edits

For a v4 config, write these options when the corresponding setting was absent. The example shows the compatibility values; generated settings also receive the comments described below.

```ts
export default defineConfig({
  test: {
    clearMocks: false,
    sharedViteServer: false,
    projects: [
      {
        extends: false,
        test: {
          clearMocks: false,
          browser: {
            locators: { exact: false },
          },
        },
      },
    ],
  },
});
```

Only write `sharedViteServer: false` when the config has inline projects. Only write `extends: false` on inline object projects that omitted it. Write `clearMocks: false` into each effective project that does not inherit the root setting. Only write `browser.locators.exact: false` for browser projects. Preserve explicit settings and inherited values. When an external base cannot be resolved, retain the child's settings and report inheritance for review instead of inserting defaults that can override the base.

Beside each added compatibility setting, write a comment with its reason, guidance for adopting v5 behavior, and a link to the relevant upstream migration section. This includes `clearMocks`, `sharedViteServer`, `extends`, locator `exact`, coverage `perFile`, and Temporal `toNotFake`. Leave existing settings and comments untouched, including settings inserted by earlier migrations. Repeated runs must not duplicate comments or restore comments that the user removed. See the [migration guide](../docs/guide/vitest-v5.md#preserve-existing-behavior) for an example.

If a v4 project has no test config, use v5 defaults without a review prompt or a new compatibility config. If another migration step creates `vite.config.ts`, apply the v4 compatibility settings to that config during the same invocation.

Also apply these edits when no conflict exists:

- move `browser.api` to top-level `api`, or remove the redundant browser setting when both static values are equivalent, regardless of property order or quote style;
- copy a custom `browser.screenshotDirectory` to `browser.expect.toMatchScreenshot.screenshotDirectory`;
- add `perFile: true` to glob threshold objects that inherited a top-level `coverage.thresholds.perFile: true` in v4;
- add `.vitest/` to `.gitignore`, but retain old artifact entries until their directories are empty;
- add `Temporal` to `fakeTimers.toNotFake` when the project installs a global Temporal polyfill and did not configure the option.

The `toNotFake` option only preserves behavior while fake timers run. It does not preserve the v4 behavior of `vi.setSystemTime()` without fake timers.

### Safe source and config rewrites

Apply AST or structured command rewrites for these forms:

- `test.sequential`, `it.sequential`, `describe.sequential`, and `{ sequential: true }` to `{ concurrent: false }` forms, moving a numeric third-argument timeout into the new options object;
- unawaited async assertions in tests and hooks to awaited forms when their callbacks can safely become `async`;
- all v4 browser `toHaveTextContent` calls with a string or regular expression to `toMatchTextContent`;
- `await render(...)` for `vitest-browser-vue` and `vitest-browser-svelte` when the enclosing callback can become `async` without changing its contract;
- `resolveConfig` pair destructuring to the v5 return value and `.test` access;
- deprecated Vitest entry points to the canonical Vite+ v5 paths;
- root-safe `@vitest/expect` imports to `vite-plus/test`;
- supported `@vitest/runner` uses to `TestRunner` from `vite-plus/test`;
- direct `vitest list` commands without a static-parse flag to include `--no-static-parse`;
- direct calls to the programmatic `Vitest.collect()` API to pass `{ staticParse: false }` when the options are absent or are a static object without `staticParse`.

Run specific mappings before the current generic `vitest/<subpath>` rewrite. The generic rule must only emit a path present in the final `vite-plus` export map.

Do not infer exact text-match intent from a v4 browser `toHaveTextContent` call. A string used partial matching in v4, even when the current fixture contains the same complete string. A browser project can keep `toHaveTextContent` only when the user chooses v5 assertion behavior.

Preserve Node `@testing-library/jest-dom` calls. Determine file ownership before renaming a plain `expect()` matcher. Report shared Node/browser files, dynamic project settings, and browser CLI overrides for review. An `expect.element()` call identifies a browser assertion even when the config is dynamic.

Follow `projects` and `extends` references within test configs, excluding plugin options with those names. Report the root jest-dom type entry for review in Vitest TypeScript configs. Users must load `@testing-library/jest-dom/vitest` through `compilerOptions.types` or an included TypeScript setup file; a JavaScript runtime setup alone does not prove type coverage.

Use an explicit symbol allowlist for `@vitest/runner` migrations:

| v4 runner symbol                                                                                                       | v5 migration                                                                                                                                |
| ---------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `test`, `it`, `describe`, `suite`, `recordArtifact`, `TestAPI`, `SuiteAPI`, `SuiteCollector`, `TestArtifact`           | import the same name from `vite-plus/test`                                                                                                  |
| `File`, `Suite`, `Test`, `Task`, `VitestRunner`, `VitestRunnerConfig`                                                  | import `RunnerTestFile`, `RunnerTestSuite`, `RunnerTestCase`, `RunnerTask`, `VitestTestRunner`, or `TestRunnerConfig` from `vite-plus/test` |
| `getCurrentSuite`, `getCurrentTest`, `createTaskCollector`                                                             | use `TestRunner.getCurrentSuite`, `TestRunner.getCurrentTest`, or `TestRunner.createTaskCollector` from `vite-plus/test`                    |
| `getFn`, `getHooks`, `setFn`, `setHooks`                                                                               | use `TestRunner.getTestFn`, `TestRunner.getSuiteHooks`, `TestRunner.setTestFn`, or `TestRunner.setSuiteHooks` from `vite-plus/test`         |
| `FileSpecification`, `SuiteHooks`, `TaskUpdateEvent`, `startTests`, `collectTests`, `updateTask`, and unlisted symbols | report the use and require manual migration                                                                                                 |

The migration must preserve local import aliases when it applies a supported rewrite. An unsupported runtime use blocks the package update. An unsupported type-only use produces a review-required finding.

### File ownership

Build scopes from active configs, not every discovered config file. Respect `vitest.config` precedence over `vite.config`, explicit config selection in scripts, and referenced projects. A sibling inactive config must not suppress global API migration.

Resolve `test.root` as an override of the Vite root. Track `test.dir` separately as the test discovery directory, including literal `--root` and `--dir` overrides. Match test include/exclude patterns relative to that directory. Keep setup-file resolution tied to the owning project's root, independently of test discovery patterns.

Include declared setup files even when they have no test filename or Vitest import. Resolve extensionless entries with Vitest-compatible extension precedence and directory index lookup. Report unresolved entries rather than assigning ownership to a guessed file.

Resolve benchmark membership separately through `benchmark.include`, `benchmark.exclude`, and `benchmark.includeSource`, including default benchmark filenames. Scan known in-source benchmarks with `import.meta.vitest` even when globals are disabled.

Only rewrite unbound global APIs in files owned by compatible Vitest scopes. Explicit Vitest bindings remain usable independently of global settings. Preserve unrelated Jest or other runner files. Report uncertain ownership when dynamic settings or incompatible shared scopes prevent a safe decision.

### Review-required findings

Report remaining issues with a file location and documentation link. Gate reviews about changed v4 behavior on the original source version; do not repeat them for a project already on v5. Removed APIs and other invalid current code still need diagnostics. The checklist covers:

- `-t` patterns that may span a suite boundary, excluding plain single-segment filters;
- nested `vi.mock`, `vi.unmock`, or `vi.hoisted` calls;
- factory-free `vi.mock()` calls in browser tests;
- class constructor mocks created with `vi.fn`, `vi.spyOn`, or `mockImplementation`;
- benchmark references, options, and removed flags that remain after supported automatic rewrites;
- unawaited `resolves`, `rejects`, file-snapshot, poll, or browser assertions that cannot be rewritten safely;
- custom matcher declarations that use the old `Assertion<T>`, `Matchers<T>`, or `jest.Matchers` shape;
- custom browser commands that receive a locator string;
- code that uses `VITEST_POOL_ID` or `VITEST_WORKER_ID` as a zero-based value;
- custom environments that restore `populateGlobal().originals` with assignment;
- assignments to DOM globals in jsdom or happy-dom tests;
- Temporal use in a file or setup scope that also calls `vi.setSystemTime()`;
- direct UI or browser-orchestrator URLs without a token or session ID;
- referenced config files that merge a root config containing `test.projects`;
- dynamic, function, or promise inline projects whose effective config cannot be determined from source;
- plugins that depend on one Vite server or config execution per project;
- `vitest list` commands or programmatic `collect()` calls whose static-parse setting cannot be determined or rewritten safely;
- configured `coverage.include` or `coverage.exclude` when a v4/v5 resolved file-set comparison is unavailable;
- imports from removed runner, suite, or internal module-runner APIs that have no direct replacement;
- `@vitest/expect` or `@vitest/runner` symbols that are not available from the root v5 API.

The command exits successfully after safe edits when only review items remain. Block dependency updates when the source version cannot be determined, when conflicting or unresolved `api` values cannot be merged, or when an unsupported removed runtime API remains. Incompatible Node pins are automatic edits, not blockers; unresolved Node selectors require review.

### Exact output and removed-API reference

Keep these v5 defaults as a reference for users who consume reports and artifacts outside Vitest. They do not require output-only migration edits or diagnostics:

| Surface               | v4                              | v5                                                     |
| --------------------- | ------------------------------- | ------------------------------------------------------ |
| Attachments           | `.vitest-attachements/`         | `.vitest/attachments/`                                 |
| Failure screenshots   | `__screenshots__/`              | `.vitest/attachments/failure-screenshots/`             |
| Blob reporter         | `.vitest-reports/blob-*.json`   | `.vitest/blob/blob-*.json`                             |
| HTML reporter         | `html/index.html`; `outputFile` | `.vitest/index.html`; `outputDir`                      |
| JSON reporter         | stdout                          | `.vitest/json/output.json`                             |
| JUnit reporter        | stdout                          | `.vitest/junit/output.xml`                             |
| Reference screenshots | `browser.screenshotDirectory`   | `browser.expect.toMatchScreenshot.screenshotDirectory` |

Users can retain JSON and JUnit stdout with `{ stdout: true }`. An explicit reporter `outputFile` remains valid except for the HTML reporter's renamed directory option. The migration does not insert `stdout: true` or rewrite regular HTML reporter destinations.

Vitest v5 removes the top-level `bench` API; `bench.skip`, `bench.only`, and `bench.todo`; `benchmark.reporters`; `benchmark.outputFile`; `benchmark.compare`; `benchmark.outputJson`; `--compare`; and `--outputJson`. The replacements are the `bench` test-context fixture, regular test modifiers, and regular reporters. `Vitest.mode` is always `test`.

Convert direct `bench` calls with inline zero-argument callbacks or references to unchanged local functions into tests using the `bench` fixture and `.run()`. Support dynamic names by evaluating them once during registration. Preserve parenthesized callback arguments and surrounding `describe` and `suite` scopes. Move simple `skip`, `only`, and `todo` modifiers to the test. Use readable `test` and `bench` bindings, adding aliases only when needed to avoid a name conflict.

Apply these rewrites to imports, owned globals, and supported destructured or direct-member uses of `import.meta.vitest`. Preserve the in-source guard and production behavior. Block unresolved references, wrappers, calls with benchmark options, and unsupported callbacks. If syntax validation or overlapping edits discard a rewrite, retain the removed-API blocker; a review alone must not permit upgrading unchanged legacy code.

Move built-in `benchmark.reporters` settings (`default` and `verbose`) and literal `benchmark.outputFile` or `benchmark.outputJson` destinations to regular reporters when compatible. Rewrite literal `--outputJson` commands to use the JSON reporter while preserving the path. Keep existing reporter choices when they conflict with the proposed move and report the unresolved setting. Custom reporters, dynamic settings, project-specific settings, and config merges need manual review or a blocker when a removed option remains.

Retain supported benchmark options, including `benchmark.enabled`, discovery patterns, and the `vitest bench` command. Baseline `compare` settings and `--compare` flags remain blockers. Report consumers must adopt the v5 JSON schema themselves; do not emit an output-format-only diagnostic.

The entry-point migration uses these exact mappings:

| Removed v4 entry point                   | v5 action                                      |
| ---------------------------------------- | ---------------------------------------------- |
| `vitest/coverage`, `vitest/reporters`    | import from `vite-plus/test/node`              |
| `vitest/environments`, `vitest/snapshot` | import from `vite-plus/test/runtime`           |
| `vitest/runners`                         | import `TestRunner` from `vite-plus/test`      |
| `vitest/suite`                           | use static methods on `TestRunner`             |
| `vitest/mocker`                          | import from `vite-plus/test/mocker`            |
| `vitest/internal/module-runner`          | no public replacement; require manual redesign |

## Complete incompatible-change and risk matrix

This matrix tracks every v5 migration-guide item and the extra breaking entries in the release notes. It includes reference-only output changes that do not produce migration findings. “Scan” refers to the versioned `vp migrate` pass; reviews about changed v4 behavior apply only when migrating from v4.

| Change                                                       | Upgrade risk                                                                                                                                            | Vite+ handling                                                                                                                                  |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| Node `>=22.12` and Vite `>=6.4`                              | The local CLI can fail before tests start.                                                                                                              | Raise the CLI engine, migrate incompatible Node declarations, and keep bundled Vite in range.                                                   |
| `clearMocks: true` default                                   | Setup-file, top-level, `beforeAll`, and cross-test mock history disappears.                                                                             | Add commented `clearMocks: false` to v4 configs when absent; preserve explicit and inherited settings.                                          |
| Configless v4 projects                                       | New defaults apply without a config file that the migration can edit.                                                                                   | Use v5 defaults without a prompt or a new compatibility config; apply v4 defaults if another step creates a config.                             |
| Full test names use `>`                                      | `-t 'suite test'` no longer matches across the boundary.                                                                                                | Review filters that may span a boundary; do not report plain single-segment filters.                                                            |
| Test listing uses static parsing                             | Runtime-generated tests and collection side effects can be absent from `vitest list` and `Vitest.collect()` results.                                    | Add `--no-static-parse` or `{ staticParse: false }` to direct migrated uses; report dynamic wrappers.                                           |
| Browser iframe scaling changed                               | Headed browser UI and screenshot dimensions can differ from v4.                                                                                         | Run headed and headless screenshot fixtures at fixed viewports; refresh baselines only after review.                                            |
| Inline projects inherit root config                          | Plugins, aliases, setup files, and arrays can apply twice or begin applying.                                                                            | Add `extends: false` to static entries, report dynamic entries, and make plugin injection inheritance-aware.                                    |
| Referenced configs can define nested projects                | A merged root `projects` field can recurse, duplicate projects, or resolve paths from a new base.                                                       | Scan referenced configs and root-config merges; require manual extraction of a shared config.                                                   |
| Inline projects share a Vite server                          | Config files and plugin hooks run fewer times; stateful plugins can change behavior.                                                                    | Add `sharedViteServer: false` for migrated inline-project configs; new projects use sharing.                                                    |
| Hoisted mock calls must be top-level                         | A prior warning becomes a startup error.                                                                                                                | Report each nested call; do not move it automatically because scope dependencies may change.                                                    |
| Browser automocks remain mocked                              | Exports that called real code now return mock defaults.                                                                                                 | Report factory-free browser mocks; suggest `{ spy: true }` when real behavior is required.                                                      |
| Class mocks inherit implementation prototypes                | Methods and `instanceof` results change.                                                                                                                | Add focused release tests; report class constructor mocks as review items.                                                                      |
| Benchmark API rewrite                                        | Top-level `bench` and benchmark-specific reporter, output, and compare options are removed; `vitest bench` remains supported.                           | Convert supported imported, global, and in-source calls, reporters, and output flags; block retained removed APIs and comparisons.              |
| UI token authentication                                      | Stored or proxied bare UI URLs stop working.                                                                                                            | Preserve the token URL in CLI output; report hard-coded `/__vitest__/` URLs.                                                                    |
| Fake timers mock `Temporal`                                  | Time tests with a global polyfill change.                                                                                                               | Preserve v4 fake-timer behavior with `toNotFake: ['Temporal']` when detected.                                                                   |
| `vi.setSystemTime()` mocks `Temporal`                        | Code can observe mocked Temporal time without enabling fake timers.                                                                                     | Report scopes that contain both Temporal use and `vi.setSystemTime()`; do not rewrite them.                                                     |
| `toThrow('')` matches any message                            | Assertions for an empty error message become too broad.                                                                                                 | Rewrite `toThrow('')` and `toThrowError('')` to `/^$/`.                                                                                         |
| Assertion types add return and received parameters           | Custom matcher declarations and direct assertion types fail type checking.                                                                              | Report old generic forms, update Vite+ examples, and add type fixtures.                                                                         |
| `expect.poll` rejects at timeout                             | Polls that completed late now fail.                                                                                                                     | Report configured polls and ask users to review the timeout; do not raise it automatically.                                                     |
| Unawaited async assertions fail                              | Tests that passed with a warning now fail.                                                                                                              | Detect expression statements and add `await` only when the transform is safe; report the rest.                                                  |
| Titles and inspected values use `pretty-format`              | Snapshots, `test.each` titles, and reporter consumers can change.                                                                                       | Use v5 formatting without migration edits or prompts.                                                                                           |
| Sequential APIs and options are removed                      | Test collection fails or concurrency changes.                                                                                                           | Rewrite to `{ concurrent: false }` and preserve numeric timeout arguments in the options object.                                                |
| Browser command locators serialize as objects                | Custom commands receive an object instead of a selector string.                                                                                         | Report locator-typed command parameters and suggest `SerializedLocator`.                                                                        |
| Browser locators are exact by default                        | Partial or case-insensitive text lookups stop matching.                                                                                                 | Add `browser.locators.exact: false` to migrated browser projects.                                                                               |
| `toHaveTextContent` is strict                                | Partial strings and regular expressions fail.                                                                                                           | Rewrite all v4 string and regular-expression calls to `toMatchTextContent` unless the user chooses v5 semantics.                                |
| Vue and Svelte browser `render` are async                    | Immediate queries can race or access a promise.                                                                                                         | Add `await` where safe and report non-async call sites.                                                                                         |
| Glob thresholds no longer inherit `perFile`                  | Coverage enforcement can become weaker.                                                                                                                 | Copy `perFile: true` into existing glob threshold objects.                                                                                      |
| `coverage.thresholds.perFile` also accepts an object         | Config libraries that assume a boolean can reject or misread the new shape.                                                                             | Update Vite+ config types and serializers; the old boolean form needs no migration.                                                             |
| Coverage include/exclude matching is precise                 | The measured file set can shrink or change.                                                                                                             | Compare resolved v4/v5 file sets. If comparison is unavailable, report all configured patterns for review.                                      |
| Parent config lookup is removed                              | Running below the config root can ignore configuration.                                                                                                 | Document explicit `--config` and `--dir`; preserve v5 lookup rules without a startup diagnostic.                                                |
| DOM global assignment updates the window                     | `matchMedia` and other DOM APIs can observe new values.                                                                                                 | Cover jsdom and happy-dom fixtures; report assignments to known DOM globals.                                                                    |
| `populateGlobal().originals` contains descriptors            | Custom environment teardown can restore descriptor objects as values.                                                                                   | Report assignments from `originals`; recommend `Object.defineProperty`.                                                                         |
| Browser orchestrator URLs need a session                     | Direct `/__vitest_test__/` links fail.                                                                                                                  | Report hard-coded URLs and retain the URL printed or opened by Vitest.                                                                          |
| `browser.api` moves to `api`                                 | Custom ports are ignored.                                                                                                                               | Move the option or deduplicate equivalent static values; block conflicting or unresolved values.                                                |
| Reports and artifacts move under `.vitest`                   | CI uploads, merges, ignores, and stdout pipes can break.                                                                                                | Document paths and add `.vitest/` to ignore files; do not rewrite regular reporters or report output-only changes.                              |
| Screenshot references use a dedicated option                 | Existing baselines can be read from the wrong directory.                                                                                                | Copy the old custom directory to the new expectation option and retain files.                                                                   |
| Worker and concurrency IDs are 1-based                       | Array indexes, ports, and database names shift.                                                                                                         | Report environment-variable reads and require project-specific review.                                                                          |
| Worker-start failures are reported gracefully                | Wrappers that match the old thrown error or localStorage warning can observe different diagnostics.                                                     | Snapshot the Vite+ failure path and preserve Vitest's exit status; no project source migration is needed.                                       |
| `resolveConfig` returns resolved Vite config                 | Destructuring returns `undefined`; consumers miss `.test`.                                                                                              | Apply a targeted AST rewrite and add a programmatic API fixture.                                                                                |
| Runner, expect, WebSocket, and WebDriverIO package migration | Removed runner publication, split expect state, and community provider ownership can cause resolution or state failures; WebSocket APIs are deprecated. | Migrate supported APIs, report unsupported uses, route assertions through root `vitest`, and decouple WebDriverIO; no deprecation-only warning. |
| Deprecated entry points are removed                          | Generated Vite+ shims and current generic migration output can become invalid.                                                                          | Keep exact compatibility aliases, rewrite to canonical paths, and block unsupported internals.                                                  |

## Rollout

### Phase 1: compatibility branch

- Pin the official Vitest packages to `5.0.1` in a development branch.
- Update the package graph, export generator, resolver, project plugin injection, and migration package set.
- Add export snapshots and unit fixtures before accepting generated package changes.
- Preserve the root Vitest pin during upstream catalog synchronization and align Vite's direct test dependencies.

### Phase 2: migration support

- Implement the versioned scan, safe rewrites, commented compatibility options, and automatic Node pin upgrades.
- Add before-and-after fixtures for v4 and v5 projects, configless projects, workspaces, browser projects, and stateless reruns.
- Cover active config selection, mixed-runner ownership, setup files, and benchmark discovery before allowing global API edits.
- Publish the migration guide with each item in the risk matrix.

### Phase 3: Vite+ prerelease

- Confirm that the lockfile resolves the official `5.0.1` package graph.
- Verify one compatible community WebDriverIO provider against Vitest `5.0.1`.
- Publish a Vite+ prerelease and run ecosystem CI against real projects.
- Keep the v4-based Vite+ release available for Node 20 and for any WebDriverIO user blocked by community-provider timing.

### Phase 4: stable release

Release only after the gates below pass. State the Node requirement and the `vp migrate` command in the release notes. Do not remove the exact legacy aliases listed in this RFC during the Vite+ 1.x line.

## Validation and release gates

Run these checks against the release-candidate commit with Vitest `5.0.1`. Record the commit, runtime and provider versions, and CI results. The historical results in the appendix describe earlier revisions and do not establish that the current candidate passes.

The implementation must pass these gates:

1. `pnpm tsgo`, `vp check`, `pnpm test:unit`, Rust checks for changed global-CLI code, and the CLI snapshot suite.
2. Package export tests that resolve every generated `vite-plus/test*` path under its published conditions and compile all typed paths with TypeScript. Import Node-facing APIs under Node ESM, test-facing APIs inside Vitest, and browser-only APIs inside a browser test. Type-only exports and upstream context-error stubs must retain their upstream behavior; importing a browser-only API in plain Node is not a success condition.
3. Identity tests that prove `vp test`, `vite-plus/test`, browser providers, custom matchers, and coverage use one runner and assertion state.
4. Node 22.18, 24.11, and 26 jobs. Cover command preparation without a Node-version probe and automatic pin upgrades through `vp migrate`.
5. npm, pnpm, Yarn, and Bun install and test fixtures. Vite+ does not support Yarn PnP runtime resolution. Start the Yarn fixture with PnP, run the documented `vp migrate` conversion to `node_modules`, then execute its tests. Adding PnP runtime support is outside this upgrade.
6. Configless, single-project, inherited inline-project, `extends: false`, referenced-config, nested-project, and shared-server fixtures. Add a fixture that combines `extends: false` with `sharedViteServer: false`. Assert independent server and plugin-hook behavior without extra inheritance. Cover config precedence, root and directory overrides, setup resolution, and test/benchmark ownership with preservation checks for unrelated files.
7. Playwright and Preview browser suites. Track the existing Preview real-timer regression separately from new v5 failures, as described below. Run the WebDriverIO suite against a verified community release without requiring an exact Vitest patch version.
8. V8 and Istanbul coverage with matching providers, mismatched-provider rejection, glob threshold checks, and v4/v5 file-list comparison. Verify the final `@vitest/istanbul-lib-*` dependency graph.
9. JSON, JUnit, HTML, blob merge, attachments, failure screenshots, and reference-screenshot path fixtures.
10. UI token, browser session, custom command locator, jsdom, happy-dom, Temporal, custom environment, worker ID, `resolveConfig`, `vitest list`, `Vitest.collect()`, custom matcher, and benchmark migration fixtures. Include globals and `import.meta.vitest` benchmarks, parenthesized callbacks, and plans that retain blockers after rejected rewrites.
11. `ecosystem-ci` cases for `vite-plus-vitest-global-type-minimal-repro`, `vitest-playwright-repro`, and `vite-plus-vitest-type-aug`, followed by the broader ecosystem set.
12. Dependency synchronization and a frozen-lockfile install from a clean checkout. The alignment helper must run on CI's preinstalled Node before Node setup or dependency installation. Upstream Vite and Rolldown test suites are outside this release gate.
13. Migration diagnostics with documentation links, a single final review summary, automatic Node pin upgrades, and comments beside newly inserted compatibility settings. Verify v5-to-Vite+ migration and stateless reruns without v4 defaults, v4-only reviews, or output-only prompts.

## Alternatives

### Adopt all v5 defaults for existing projects

This keeps migrated configs smaller but makes an ordinary package update change mock history, project inheritance, plugin lifetimes, and locator matching at once. New and configless projects get these defaults. Existing v4 configs receive explicit compatibility settings that users can remove after the affected tests pass.

### Force v4 defaults inside `vp test`

This avoids config edits but makes Vite+ behavior differ from the Vitest v5 documentation and programmatic API. Explicit migration options are visible and removable.

### Mirror every v4 package and entry point

Several removed runner and suite APIs do not have a complete v5 target. Partial shims would fail later and with less useful errors. Keep only aliases with exact public replacements.

### Drop WebDriverIO support

The provider kept its package name and moved to community maintenance. An optional peer and compatibility shim preserve current Vite+ imports without coupling release versions. Drop the shim only if the community package cannot pass the v5 browser contract before the Vite+ stable release.

## Known upstream limitation: Preview real timers

On September 15, 2026, the same unmodified locator-click test passed with Vitest and Preview `4.1.0`, then failed with `4.1.1`, `4.1.11`, and `5.0.0`. Each run used `npx vitest run`, Vite `8.3.0`, and real timers. The [standalone reproduction](https://github.com/why-reproductions-are-required/vitest-5-preview-real-timers) also reproduces the `5.0.0` failure in a CLI-only GitHub Action.

[Vitest #9891](https://github.com/vitest-dev/vitest/pull/9891), shipped in [`4.1.1`](https://github.com/vitest-dev/vitest/releases/tag/v4.1.1), added `vi.advanceTimersByTimeAsync()` to the Preview user-event adapter without a fake-timer check. The source v4 baseline is `4.1.11`, which has the same bug. It is not a new v5 regression or a v5 release blocker, so migration does not add a warning or change provider or timer settings for it.

The `test_v5_preview` fixture records the known failure separately from the passing fake-timer integration case. Keep the failure check until upstream fixes it, then require the real-timer test to pass. Passing the expected-failure check does not prove that real-timer clicks work. Use Playwright for affected tests; an arbitrary v4 release or forced fake timers is not a fix.

## Release follow-up

1. Verify community `@vitest/browser-webdriverio@5.0.0` against Vitest `5.0.1`. Earlier prerelease-provider results do not replace this check. If it fails, document the provider exception and keep affected users on the v4-based release.
2. Verify a clean `--frozen-lockfile` install after the shared helper aligns Vite's dependencies with the root Vitest pin. Keep this installation check without requiring upstream Vite or Rolldown test-suite migrations.
3. Verify the upstream browser `define` fix in `5.0.1`, with no Vite+ backport. Keep `test_v5_preview` and `test_v5_browser_defines` to check strings and booleans in raw configs, inherited projects, programmatic APIs, and packed installations. Rerun affected ecosystem projects with the upstream release.
4. Check Node jest-dom and browser matcher declarations together. The recorded `5.0.1` probe rejected Node regex text assertions and CSS custom-property assertions when `vitest/browser` loaded before the jest-dom augmentation; reversing the imports passed. The `test_v5_upstream_types` fixture covers both orders with `@testing-library/jest-dom` versions `6.9.1` and `7.0.1`, without Vite+ installed. Resolve the declaration conflict before enabling the upgrade for affected projects; an explicit jest-dom type entry alone did not fix the recorded `dify` failure.

## Appendix: Historical validation

These results record earlier branch revisions from September 2026. They predate the final migration scope, diagnostic policy, and CI bootstrap changes. Counts and failures below are historical evidence, not the current release status.

### Prerelease investigation

The investigation spike used `5.0.0-rc.2`. It removed the unavailable runner package, built the CLI and 60 generated test exports, and passed the focused config/resolver tests. The complete TypeScript unit run passed 63 files and 1,029 tests. The spike also reproduced the vendored Rolldown catalog conflict. The `rc.4` static-collection change was not present in that spike.

### Earlier implementation checks

Local checks on macOS recorded the following results:

- Node `22.18.0`, `24.11.0`, and `26.0.0`: 70 unit-test files, with 1,240 tests passed and one skipped on each runtime. Type checking, `vp check`, and the docs build passed.
- Rust: 314 migration tests, the global-CLI Node-range test, and 24 snapshot-redaction tests passed. `cargo fmt --all --check` and Clippy with warnings denied passed for the changed migration and global-CLI crates.
- The September 6 upstream-suite run passed 1,904 tests and skipped 19 across Vite, Rolldown, Rolldown watch, and dev-server fixture suites. The `test:vendored` command and its CI step were later removed; these upstream suites are outside the scope of this upgrade.
- A full CLI snapshot rerun passed 774 cases with one ignored. Three upstream-failure regression cases passed separate comparison runs; they verified expected failures, not successful browser behavior. That suite included five package-manager installation cases and covered mixed Node/browser assertions, plugin config references, container runtime pins, jest-dom type guidance, and Yarn without project `vite` or `vitest` dependencies. Container runtime checks were later removed from migration scope.
- The three required ecosystem projects passed their prescribed test or type-check commands against the packed checkout. Additional runs covered `vite-plus-jest-dom-repro`, `viteplus-ws-repro`, `vp-config`, and `vite-vue-vercel`. The complete `oxlint-plugin-complexity` workflow passed with lint warnings, including 494 tests. The `vite-plus-monorepo-overrides` check and verification workflow also passed.
- `tanstack-start-helloworld` passed eight browser tests and its build. `bun-vite-template` passed its validation workflow after the fixture applied the reported jest-dom type-entry repair. On Node `22.18.0`, `rollipop` passed its builds, formatting, and 24 tests; its report-only lint and type-check steps reported errors.
- On Node `24.11.0`, `vibe-dashboard` passed formatting, five browser tests, and its build. `nuxt-devtools` passed its build, type check, and 67 tests. Its migration reported an automatic-formatting failure; its prescribed workflow did not include formatting.
- `reactive-resume` passed formatting, type-aware lint, its build, and 196 tests on Node `24.11.0`. On the same runtime, `vitepress` passed formatting, its build, 69 unit tests, 35 development-mode browser tests, 34 production-mode browser tests, and six initialization tests; one production-mode test was skipped.
- `frm-stack` passed lint, formatting, type checking, and 24 Docker-backed database tests on Node `24.11.0`. `tiptap` passed its package and demo builds, lint, and 1,920 tests on that runtime. On Node `22.18.0`, `varlet` passed its bootstrap and 921 Istanbul coverage tests; its report-only lint step reported the documented ambient `declare` error.
- `vinext` passed its build and checks on Node `24.11.0`. Its unit shard passed 4,615 tests with two skipped on Node `24.20.0`, with loopback requests excluded from the local proxy. The earlier `24.11.0` run exposed Node glob and React stream differences; tests that used `localhost` also failed until the proxy exclusion was set.
- `dify` passed its build and 41 tests in two files selected by the prescribed test filters. Its build skipped type validation; the separate type check failed on the Node/browser matcher conflict. `npmx.dev` passed formatting, type checking, and 1,652 Node tests. Its browser run passed 819 tests, failed 254, and skipped five; malformed translation-resource URLs reproduced the browser `define` issue. Its lint step was report-only.
- The WebDriverIO fixture passed with community provider `5.0.0-rc.1`. Preview passed with fake timers; locator clicks with real timers reproduced the existing upstream regression.

At that revision, 20 of 25 active ecosystem projects passed their prescribed workflows. Preflight blocked `decoders` on its Node `20.x` CI pin, `vue-mini` on its Node `25.9.0` runtime pin, and `yaak` on its Node `20` pin. Those blockers do not describe the current migration: CI Node declarations are now out of scope, and incompatible pins in supported locations are upgraded automatically. The `dify` and `npmx.dev` failures, and the outstanding Linux, Windows, and published-prerelease checks, also describe that revision rather than the current candidate.

### Browser define backport check

The September 11 `npmx.dev` run with the temporary backport passed 1,028 browser tests and skipped five; 45 failures in `a11y.spec.ts` reported Vue's `decodeEntities` warning. Vitest `5.0.1` later supplied the upstream fix, so the Vite+ backport was removed. This earlier run does not replace validation against that release.
