# RFC: Upgrade the Bundled Vitest to v5

- Status: Proposed
- Tracking issue: [#2405](https://github.com/voidzero-dev/vite-plus/issues/2405)
- Upstream baseline: [`v5.0.0`](https://github.com/vitest-dev/vitest/releases/tag/v5.0.0)
- Release target: Vite+ before `1.0`

## Decision

Upgrade `vp test` and the `vite-plus/test*` API to Vitest `5.0.0` before Vite+ 1.0. Pin the official Vitest packages to the final release.

The upgrade has four parts:

1. Update the bundled runner, browser packages, public shims, resolver, and Vite config integration as one versioned unit.
2. Add a Vitest v5 pass to `vp migrate`. It preserves v4 behavior where a config option can do so, applies safe source rewrites, and reports changes that need review.
3. Raise the `vite-plus` Node.js range to the intersection of the current Vite+ range and the Vitest v5 range: `^22.18.0 || ^24.11.0 || >=26.0.0`.
4. Release the change through a Vite+ prerelease and run the compatibility matrix in this RFC before a stable release.

New projects use the Vitest v5 defaults. Migrated projects get explicit compatibility options. A user can remove those options later to adopt the new defaults one at a time.

## Context

Vite+ currently pins `vitest` and the managed `@vitest/*` family to `4.1.11` in `pnpm-workspace.yaml`. The `vite-plus` package:

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
- Preserve v4 behavior for existing projects when a v5 option provides an exact compatibility setting.
- Give a file and an action for each change that cannot be automated safely.
- Keep useful Vite+ legacy aliases through the Vite+ 1.x line when they have an exact v5 target.
- Test Node, package manager, project, browser, coverage, reporter, and programmatic API boundaries before release.

## Non-goals

- Reimplement removed Vitest internals.
- Make the v5 benchmark rewrite automatic.
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

## Compatibility design

### 1. Runtime and dependency graph

Pin the official Vitest packages to the exact version `5.0.0`. Keep the existing coverage-provider version guard. Coverage packages remain project-installed peers and must match the bundled runner exactly.

| Package group                                                                                 | Policy                                                                                                                                                                                                                                |
| --------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vitest`                                                                                      | Exact dependency and the only runner started by `vp test`                                                                                                                                                                             |
| `@vitest/browser`, `@vitest/browser-playwright`, `@vitest/browser-preview`                    | Exact version equal to the runner                                                                                                                                                                                                     |
| `@vitest/mocker`, `@vitest/pretty-format`, `@vitest/snapshot`, `@vitest/spy`, `@vitest/utils` | Keep only when a public Vite+ shim needs the standalone package; pin exactly                                                                                                                                                          |
| `@vitest/expect`                                                                              | Remove from the bundled graph; route Vitest assertions through the root `vitest` entry so assertion state stays shared                                                                                                                |
| `@vitest/runner`                                                                              | Remove from catalogs, dependencies, shims, migration alignment, and resolver assumptions; v5 no longer publishes it with the runner                                                                                                   |
| `@vitest/ws-client`                                                                           | Do not add a Vite+ shim; report direct use because upstream deprecated it and will not add v5 features                                                                                                                                |
| `@vitest/browser-webdriverio`                                                                 | Keep as an optional compatibility peer backed by the [community repository](https://github.com/vitest-community/vitest-webdriverio); use a compatible range and test a selected version instead of forcing the runner's exact version |
| `@vitest/coverage-v8`, `@vitest/coverage-istanbul`, `@vitest/ui`                              | Project-installed optional packages; require the exact runner version                                                                                                                                                                 |
| `@vitest/web-worker`                                                                          | Project-installed optional package; keep its exact Vitest peer aligned without adding a Vite+ shim                                                                                                                                    |

Replace the resolver's broad `@vitest/*` rule with an explicit supported-package set. In particular, do not redirect a user's standalone `@vitest/expect` to a Vite+ copy. Keep bundle-first resolution for the root `vitest` package, its public subpaths, the official browser packages, and other packages that share runner state. Keep the project fallback for optional peers.

Provide Vitest's required `vite` peer through a published dependency alias to the matching `@voidzero-dev/vite-plus-core` version. Test a Yarn install with no project `vite` or `vitest` dependency; Yarn does not install a missing peer for the bundled runner.

The repository vendors Vite and Rolldown workspaces. Their catalogs still request Vitest v4, and `sync-remote` rejects the v4/v5 major conflict. Prefer upstream v5 updates. If release timing requires a local bridge, add `vitest` to the sync tool's reviewed major-conflict set, align direct `@vitest/*` dependencies in the vendored workspaces, and run their test suites with the resolved v5 graph. Do not leave a hidden mix of v4 and v5 packages in the development lockfile.

Run the bridge with `pnpm test:vendored` after building the upstream packages. It preserves the upstream suites' v4 mock-history setting and checks nine existing Vite+ branding differences without editing the vendored test files. Keep upstream Vite's `dedupeInjectedDeps: false` setting so pnpm retains `file:` dependency placement for the SSR externalization and license tests.

Vitest `5.0.0` uses packages from the `vitest-dev/istanbuljs` repository for Istanbul internals: `@vitest/istanbul-lib-coverage`, `@vitest/istanbul-lib-instrument`, `@vitest/istanbul-lib-report`, and `@vitest/istanbul-lib-source-maps`. Keep those packages on the project-installed `@vitest/coverage-istanbul` dependency edge. The resolver allowlist must not redirect them to a missing Vite+ copy.

### 2. Node.js and Vite prerequisites

Vitest v5 supports Node `^22.12.0 || ^24.0.0 || >=26.0.0` and Vite `^6.4.0 || ^7.0.0 || ^8.0.0`. The bundled Vite satisfies the Vite range.

Set the published `vite-plus` engine to `^22.18.0 || ^24.11.0 || >=26.0.0`. This removes Node 20 and Node 25 from the CLI package. Do not raise the standalone core package's engine only because of Vitest.

Before the global CLI delegates `vp test` to a local CLI, validate the selected project runtime against the local `vite-plus` engine. On failure, stop before loading Vitest and print an action such as:

```text
vp test requires Node ^22.18.0 || ^24.11.0 || >=26.0.0.
This project selects Node 20.19.0. Run `vp env pin 22 --force`, or update the project's runtime range.
```

The migration preflight must inspect `.node-version`, `.nvmrc`, `devEngines.runtime`, `engines.node`, CI matrices, and common container images. It may update a Vite+ runtime pin after confirmation. It must not silently widen a library's public `engines.node` contract.

Read official Node image tags from Dockerfile `FROM` instructions, YAML `image` or `container` fields, and Dev Container image settings. Read the Node feature's `version` option separately; its feature tag does not identify the runtime. Ignore `node:2` properties in application code and comments that mention images.

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

At the time of the final Vitest release, the latest community WebDriverIO provider is `5.0.0-rc.1`. The final Vitest manifest accepts `@vitest/browser-webdriverio` versions `^5.0.0-beta.5 || >=5.0.0`. Test the selected community version against Vitest `5.0.0`; do not hold the official package graph at an earlier release candidate.

### 4. Config integration and project inheritance

Vitest v4 inline projects did not inherit root plugins. The current Vite+ wrapper therefore prepends its three plugins to the root and every inline project. Vitest v5 sets `extends: true` by default and merges plugin arrays. The current wrapper would register each Vite+ plugin twice.

Change the injection algorithm as follows:

1. Inject the resolver, dependency-inline, and coverage-version plugins into the root config.
2. For an inline project that inherits the declaring config, rely on the inherited plugins.
3. For `extends: false` or an explicit external base config, inject the plugins into that project.
4. Apply the same rule after resolving function and promise project entries.
5. Make each Vite+ plugin idempotent by name as a defense against user merges and nested referenced configs.
6. Keep the coverage guard keyed by the shared Vitest instance because `configureVitest` can run for several projects.

Referenced config files and directories still resolve their own Vite config. Their `defineConfig` or `defineProject` call injects the Vite+ plugins. Add fixtures for raw config files that do not use the Vite+ helpers and give them the existing soft fallback behavior.

Do not restore Vitest's removed parent-directory config lookup inside the runner. If `vp test` starts below a parent config and no explicit config was passed, inspect parents only to produce this diagnostic:

```text
No test config was found in this directory.
A config exists at ../vite.config.ts. Run `vp test --config ../vite.config.ts --dir .`.
```

This keeps Vite+ aligned with the v5 CLI while giving the user a direct repair.

## Migration design

Add a versioned Vitest v5 migration pass. Run its preflight before package changes. Print the report again after edits with unresolved items grouped by file.

Determine the source runner version before updating dependencies, including catalog and installed Vite+ dependencies. Record completion of the v4 compatibility pass in the migration state, including configless projects. A repeated migration must retain explicit v5 settings and must not apply v4 defaults to projects that already use v5. Unresolved findings remain visible on subsequent scans.

Require installed or lockfile evidence when a dependency range spans both v4 and v5. Use portable workspace paths in the committed migration state. Retain deferred source-review findings without applying v4 edits again to a completed project.

### Behavior-preserving config edits

For an existing project, write these options when the related v4 option was absent:

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

Only write `sharedViteServer: false` when the config has inline projects. Only write `extends: false` on inline object projects that omitted it. Write `clearMocks: false` into each effective project that does not inherit the root setting. Only write `browser.locators.exact: false` for browser projects. Preserve explicit v5 settings.

Many v4 projects have no Vite or Vitest config. The Vitest v5 migration pass must not create `vite.config.ts` for these projects by default. It must report the `clearMocks` default change and each other applicable default change. If another Vite+ migration step creates a config, write the compatibility options into that config.

Offer a separate config-creation action after user confirmation. That action writes a minimal Vite+ config with the applicable compatibility options. Its output must state that a new config can change config discovery and project structure.

Also apply these edits when no conflict exists:

- move `browser.api` to top-level `api`;
- copy a custom `browser.screenshotDirectory` to `browser.expect.toMatchScreenshot.screenshotDirectory`;
- add `perFile: true` to glob threshold objects that inherited a top-level `coverage.thresholds.perFile: true` in v4;
- set `{ stdout: true }` for configured JSON or JUnit reporters that had no `outputFile` and therefore wrote to stdout in v4;
- add `.vitest/` to `.gitignore`, but retain old artifact entries until their directories are empty;
- add `Temporal` to `fakeTimers.toNotFake` when the project installs a global Temporal polyfill and did not configure the option.

The `toNotFake` option only preserves behavior while fake timers run. It does not preserve the v4 behavior of `vi.setSystemTime()` without fake timers.

### Safe source and config rewrites

Apply AST or structured command rewrites for these forms:

- `test.sequential`, `describe.sequential`, and `{ sequential: true }` to `{ concurrent: false }` forms;
- all v4 browser `toHaveTextContent` calls with a string or regular expression to `toMatchTextContent`;
- `await render(...)` for `vitest-browser-vue` and `vitest-browser-svelte` when the enclosing callback can become `async` without changing its contract;
- `resolveConfig` pair destructuring to the v5 return value and `.test` access;
- deprecated Vitest entry points to the canonical Vite+ v5 paths;
- root-safe `@vitest/expect` imports to `vite-plus/test`;
- supported `@vitest/runner` uses to `TestRunner` from `vite-plus/test`;
- direct `vitest list` commands without a static-parse flag to include `--no-static-parse`;
- direct calls to the programmatic `Vitest.collect()` API to pass `{ staticParse: false }` when the options are absent or are a static object without `staticParse`;
- HTML reporter `outputFile` directory settings to `outputDir`.

Run specific mappings before the current generic `vitest/<subpath>` rewrite. The generic rule must only emit a path present in the final `vite-plus` export map.

Do not infer exact text-match intent from a v4 browser `toHaveTextContent` call. A string used partial matching in v4, even when the current fixture contains the same complete string. A browser project can keep `toHaveTextContent` only when the user chooses v5 assertion behavior.

Preserve Node `@testing-library/jest-dom` calls. Determine file ownership from static project roots and include/exclude patterns before renaming a plain `expect()` matcher. Report shared files, dynamic project settings, and browser CLI overrides for review. An `expect.element()` call identifies a browser assertion even when the config is dynamic.

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

### Review-required findings

This list is the authoritative scanner checklist. Report each item with a file location. Do not claim that the scanner migrated these items:

- `-t` patterns that may span a suite boundary;
- nested `vi.mock`, `vi.unmock`, or `vi.hoisted` calls;
- factory-free `vi.mock()` calls in browser tests;
- class constructor mocks created with `vi.fn`, `vi.spyOn`, or `mockImplementation`;
- the v4 benchmark API and removed benchmark CLI flags;
- unawaited `resolves`, `rejects`, file-snapshot, poll, or browser assertions;
- custom matcher declarations that use the old `Assertion<T>`, `Matchers<T>`, or `jest.Matchers` shape;
- custom browser commands that receive a locator string;
- scripts, CI jobs, or tools that read old artifact and report paths or pipe JSON/JUnit stdout;
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
- direct `@vitest/ws-client` use and `@vitest/expect` or `@vitest/runner` symbols that are not available from the root v5 API.

The command should exit successfully after safe edits but print a high-visibility review count. It should stop before package updates for an unsupported Node range, a conflicting `api` move, or a removed API with no replacement and active runtime use.

### Exact output and removed-API reference

Migration and documentation must use the exact v5 defaults:

| Surface               | v4                              | v5                                                     |
| --------------------- | ------------------------------- | ------------------------------------------------------ |
| Attachments           | `.vitest-attachements/`         | `.vitest/attachments/`                                 |
| Failure screenshots   | `__screenshots__/`              | `.vitest/attachments/failure-screenshots/`             |
| Blob reporter         | `.vitest-reports/blob-*.json`   | `.vitest/blob/blob-*.json`                             |
| HTML reporter         | `html/index.html`; `outputFile` | `.vitest/index.html`; `outputDir`                      |
| JSON reporter         | stdout                          | `.vitest/json/output.json`                             |
| JUnit reporter        | stdout                          | `.vitest/junit/output.xml`                             |
| Reference screenshots | `browser.screenshotDirectory`   | `browser.expect.toMatchScreenshot.screenshotDirectory` |

JSON and JUnit can retain stdout with `{ stdout: true }`. An explicit reporter `outputFile` remains valid except for the HTML reporter's renamed directory option.

The benchmark migration removes the top-level `bench` API; `bench.skip`, `bench.only`, and `bench.todo`; `benchmark.reporters`; `benchmark.outputFile`; `benchmark.compare`; `benchmark.outputJson`; `--compare`; and `--outputJson`. The replacement is the `bench` test-context fixture, regular test modifiers, and the JSON reporter. `Vitest.mode` is always `test`.

Retain supported benchmark options, including `benchmark.enabled` and `benchmark.include`. Block the removed options listed above; report dynamic benchmark settings for review.

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

This matrix tracks every v5 migration-guide item and the extra breaking entries in the release notes. “Scan” refers to the versioned `vp migrate` pass.

| Change                                                       | Upgrade risk                                                                                                                                    | Vite+ handling                                                                                                              |
| ------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| Node `>=22.12` and Vite `>=6.4`                              | The local CLI can fail before tests start.                                                                                                      | Raise the CLI engine, validate the selected runtime, and keep bundled Vite in range.                                        |
| `clearMocks: true` default                                   | Setup-file, top-level, `beforeAll`, and cross-test mock history disappears.                                                                     | Add `clearMocks: false` for existing configs; use `true` for new projects.                                                  |
| Configless v4 projects                                       | New defaults apply without a config file that the migration can edit.                                                                           | Report applicable changes. Do not create a config without a separate action and user confirmation.                          |
| Full test names use `>`                                      | `-t 'suite test'` no longer matches across the boundary.                                                                                        | Scan scripts and CI strings; recommend one segment or `suite.*test`.                                                        |
| Test listing uses static parsing                             | Runtime-generated tests and collection side effects can be absent from `vitest list` and `Vitest.collect()` results.                            | Add `--no-static-parse` or `{ staticParse: false }` to direct migrated uses; report dynamic wrappers.                       |
| Browser iframe scaling changed                               | Headed browser UI and screenshot dimensions can differ from v4.                                                                                 | Run headed and headless screenshot fixtures at fixed viewports; refresh baselines only after review.                        |
| Inline projects inherit root config                          | Plugins, aliases, setup files, and arrays can apply twice or begin applying.                                                                    | Add `extends: false` to static entries, report dynamic entries, and make plugin injection inheritance-aware.                |
| Referenced configs can define nested projects                | A merged root `projects` field can recurse, duplicate projects, or resolve paths from a new base.                                               | Scan referenced configs and root-config merges; require manual extraction of a shared config.                               |
| Inline projects share a Vite server                          | Config files and plugin hooks run fewer times; stateful plugins can change behavior.                                                            | Add `sharedViteServer: false` for migrated inline-project configs; new projects use sharing.                                |
| Hoisted mock calls must be top-level                         | A prior warning becomes a startup error.                                                                                                        | Report each nested call; do not move it automatically because scope dependencies may change.                                |
| Browser automocks remain mocked                              | Exports that called real code now return mock defaults.                                                                                         | Report factory-free browser mocks; suggest `{ spy: true }` when real behavior is required.                                  |
| Class mocks inherit implementation prototypes                | Methods and `instanceof` results change.                                                                                                        | Add focused release tests; report class constructor mocks as review items.                                                  |
| Benchmark API rewrite                                        | `bench`, benchmark modes, reporters, output, and compare options are removed.                                                                   | Block on active benchmark APIs and link the new test-context fixture design.                                                |
| UI token authentication                                      | Stored or proxied bare UI URLs stop working.                                                                                                    | Preserve the token URL in CLI output; report hard-coded `/__vitest__/` URLs.                                                |
| Fake timers mock `Temporal`                                  | Time tests with a global polyfill change.                                                                                                       | Preserve v4 fake-timer behavior with `toNotFake: ['Temporal']` when detected.                                               |
| `vi.setSystemTime()` mocks `Temporal`                        | Code can observe mocked Temporal time without enabling fake timers.                                                                             | Report scopes that contain both Temporal use and `vi.setSystemTime()`; do not rewrite them.                                 |
| `toThrow('')` matches any message                            | Assertions for an empty error message become too broad.                                                                                         | Rewrite `toThrow('')` and `toThrowError('')` to `/^$/`.                                                                     |
| Assertion types add return and received parameters           | Custom matcher declarations and direct assertion types fail type checking.                                                                      | Report old generic forms, update Vite+ examples, and add type fixtures.                                                     |
| `expect.poll` rejects at timeout                             | Polls that completed late now fail.                                                                                                             | Report configured polls and ask users to review the timeout; do not raise it automatically.                                 |
| Unawaited async assertions fail                              | Tests that passed with a warning now fail.                                                                                                      | Detect expression statements and add `await` only when the transform is safe; report the rest.                              |
| Titles and inspected values use `pretty-format`              | Snapshots, `test.each` titles, and reporter consumers can change.                                                                               | Run snapshot suites and report title snapshots; do not retain the v4 formatter.                                             |
| Sequential APIs and options are removed                      | Test collection fails or concurrency changes.                                                                                                   | Apply the documented `{ concurrent: false }` rewrite.                                                                       |
| Browser command locators serialize as objects                | Custom commands receive an object instead of a selector string.                                                                                 | Report locator-typed command parameters and suggest `SerializedLocator`.                                                    |
| Browser locators are exact by default                        | Partial or case-insensitive text lookups stop matching.                                                                                         | Add `browser.locators.exact: false` to migrated browser projects.                                                           |
| `toHaveTextContent` is strict                                | Partial strings and regular expressions fail.                                                                                                   | Rewrite all v4 string and regular-expression calls to `toMatchTextContent` unless the user chooses v5 semantics.            |
| Vue and Svelte browser `render` are async                    | Immediate queries can race or access a promise.                                                                                                 | Add `await` where safe and report non-async call sites.                                                                     |
| Glob thresholds no longer inherit `perFile`                  | Coverage enforcement can become weaker.                                                                                                         | Copy `perFile: true` into existing glob threshold objects.                                                                  |
| `coverage.thresholds.perFile` also accepts an object         | Config libraries that assume a boolean can reject or misread the new shape.                                                                     | Update Vite+ config types and serializers; the old boolean form needs no migration.                                         |
| Coverage include/exclude matching is precise                 | The measured file set can shrink or change.                                                                                                     | Compare resolved v4/v5 file sets. If comparison is unavailable, report all configured patterns for review.                  |
| Parent config lookup is removed                              | Running below the config root can ignore configuration.                                                                                         | Give a parent-config diagnostic with `--config` and `--dir`; do not change v5 lookup rules.                                 |
| DOM global assignment updates the window                     | `matchMedia` and other DOM APIs can observe new values.                                                                                         | Cover jsdom and happy-dom fixtures; report assignments to known DOM globals.                                                |
| `populateGlobal().originals` contains descriptors            | Custom environment teardown can restore descriptor objects as values.                                                                           | Report assignments from `originals`; recommend `Object.defineProperty`.                                                     |
| Browser orchestrator URLs need a session                     | Direct `/__vitest_test__/` links fail.                                                                                                          | Report hard-coded URLs and retain the URL printed or opened by Vitest.                                                      |
| `browser.api` moves to `api`                                 | Custom ports are ignored.                                                                                                                       | Move the option when no top-level conflict exists; stop on conflict.                                                        |
| Reports and artifacts move under `.vitest`                   | CI uploads, merges, ignores, and stdout pipes can break.                                                                                        | Update known config and ignore files; report scripts that reference the old defaults.                                       |
| Screenshot references use a dedicated option                 | Existing baselines can be read from the wrong directory.                                                                                        | Copy the old custom directory to the new expectation option and retain files.                                               |
| Worker and concurrency IDs are 1-based                       | Array indexes, ports, and database names shift.                                                                                                 | Report environment-variable reads and require project-specific review.                                                      |
| Worker-start failures are reported gracefully                | Wrappers that match the old thrown error or localStorage warning can observe different diagnostics.                                             | Snapshot the Vite+ failure path and preserve Vitest's exit status; no project source migration is needed.                   |
| `resolveConfig` returns resolved Vite config                 | Destructuring returns `undefined`; consumers miss `.test`.                                                                                      | Apply a targeted AST rewrite and add a programmatic API fixture.                                                            |
| Runner, expect, WebSocket, and WebDriverIO package migration | Removed runner publication, deprecated WebSocket APIs, split expect state, and community provider ownership cause resolution or state failures. | Use the runner symbol allowlist, report unsupported APIs, route assertions through root `vitest`, and decouple WebDriverIO. |
| Deprecated entry points are removed                          | Generated Vite+ shims and current generic migration output can become invalid.                                                                  | Keep exact compatibility aliases, rewrite to canonical paths, and block unsupported internals.                              |

## Rollout

### Phase 1: compatibility branch

- Pin the official Vitest packages to `5.0.0` in a development branch.
- Update the package graph, export generator, resolver, project plugin injection, and migration package set.
- Add export snapshots and unit fixtures before accepting generated package changes.
- Resolve the vendored Vite and Rolldown catalog conflict without a mixed Vitest family.

### Phase 2: migration support

- Implement the versioned scan, safe rewrites, compatibility config options, and Node preflight.
- Add before-and-after fixtures for configless projects, configured projects, workspaces, and browser projects.
- Publish the migration guide with each item in the risk matrix.

### Phase 3: Vite+ prerelease

- Confirm that the lockfile resolves the final official `5.0.0` package graph.
- Verify one compatible community WebDriverIO provider against Vitest `5.0.0`.
- Publish a Vite+ prerelease and run ecosystem CI against real projects.
- Keep the v4-based Vite+ release available for Node 20 and for any WebDriverIO user blocked by community-provider timing.

### Phase 4: stable release

Release only after the gates below pass. State the Node requirement and the `vp migrate` command in the release notes. Do not remove the exact legacy aliases listed in this RFC during the Vite+ 1.x line.

## Validation and release gates

The earlier investigation spike used `5.0.0-rc.2`. It removed the unavailable runner package, built the CLI and 60 generated test exports, and passed the focused config/resolver tests. The complete TypeScript unit run passed 63 files and 1,029 tests. The spike also reproduced the vendored Rolldown catalog conflict, which confirms that dependency synchronization needs an explicit solution. Repeat these checks against final `5.0.0`; the `rc.4` static-collection change was not present in the spike.

The implementation must pass these gates:

1. `pnpm tsgo`, `vp check`, `pnpm test:unit`, Rust checks for changed global-CLI code, and the CLI snapshot suite.
2. Package export tests that resolve every generated `vite-plus/test*` path under its published conditions and compile all typed paths with TypeScript. Import Node-facing APIs under Node ESM, test-facing APIs inside Vitest, and browser-only APIs inside a browser test. Type-only exports and upstream context-error stubs must retain their upstream behavior; importing a browser-only API in plain Node is not a success condition.
3. Identity tests that prove `vp test`, `vite-plus/test`, browser providers, custom matchers, and coverage use one runner and assertion state.
4. Node 22.18, 24.11, and 26 jobs, plus actionable rejection tests for Node 20 and 25 project pins.
5. npm, pnpm, Yarn, and Bun install and test fixtures. Vite+ does not support Yarn PnP runtime resolution. Start the Yarn fixture with PnP, run the documented `vp migrate` conversion to `node_modules`, then execute its tests. Adding PnP runtime support is outside this upgrade.
6. Configless, single-project, inherited inline-project, `extends: false`, referenced-config, nested-project, and shared-server fixtures. Add a fixture that combines `extends: false` with `sharedViteServer: false`. Assert independent server and plugin-hook behavior without extra inheritance.
7. Playwright and Preview browser suites. Run the WebDriverIO suite against a verified community release without requiring an exact Vitest patch version.
8. V8 and Istanbul coverage with matching providers, mismatched-provider rejection, glob threshold checks, and v4/v5 file-list comparison. Verify the final `@vitest/istanbul-lib-*` dependency graph.
9. JSON, JUnit, HTML, blob merge, attachments, failure screenshots, and reference-screenshot path fixtures.
10. UI token, browser session, custom command locator, jsdom, happy-dom, Temporal, custom environment, worker ID, `resolveConfig`, `vitest list`, `Vitest.collect()`, custom matcher, and benchmark migration fixtures.
11. `ecosystem-ci` cases for `vite-plus-vitest-global-type-minimal-repro`, `vitest-playwright-repro`, and `vite-plus-vitest-type-aug`, followed by the broader ecosystem set.
12. Vendored Vite and Rolldown tests under the final synchronized dependency graph.

### Implementation validation (2026-09-06)

Local checks on macOS covered the following:

- Node `22.18.0`, `24.11.0`, and `26.0.0`: 70 unit-test files, with 1,240 tests passed and one skipped on each runtime. Type checking, `vp check`, and the docs build passed.
- Rust: 314 migration tests, the global-CLI Node-range test, and 24 snapshot-redaction tests passed. `cargo fmt --all --check` and Clippy with warnings denied passed for the changed migration and global-CLI crates.
- `pnpm test:vendored`: 1,904 tests passed and 19 skipped across Vite, Rolldown, Rolldown watch, and dev-server fixture suites. This command does not cover the separate upstream browser playground suites.
- The full CLI snapshot rerun passed after the closing migration and dependency fixes: 774 cases passed and one was ignored. Three upstream-failure regression cases added during that run passed separate comparison runs. They verify the failures below, not successful browser behavior. The suite includes five package-manager installation cases and covers mixed Node/browser assertions, plugin config references, container runtime pins, jest-dom type guidance, and Yarn without project `vite` or `vitest` dependencies.
- The three required ecosystem projects passed their prescribed test or type-check commands against the packed checkout. Additional runs covered `vite-plus-jest-dom-repro`, `viteplus-ws-repro`, `vp-config`, and `vite-vue-vercel`. The complete `oxlint-plugin-complexity` workflow passed with lint warnings, including 494 tests. The `vite-plus-monorepo-overrides` check and verification workflow also passed.
- `tanstack-start-helloworld` passed eight browser tests and its build. `bun-vite-template` passed its validation workflow after the fixture applied the reported jest-dom type-entry repair. On Node `22.18.0`, `rollipop` passed its builds, formatting, and 24 tests; its report-only lint and type-check steps still reported errors.
- On Node `24.11.0`, `vibe-dashboard` passed formatting, five browser tests, and its build. `nuxt-devtools` passed its build, type check, and 67 tests. Its migration reported an automatic-formatting failure; the prescribed workflow does not include formatting.
- `reactive-resume` passed formatting, type-aware lint, its build, and 196 tests on Node `24.11.0`. On the same runtime, `vitepress` passed formatting, its build, 69 unit tests, 35 development-mode browser tests, 34 production-mode browser tests, and six initialization tests; one production-mode test was skipped.
- `frm-stack` passed lint, formatting, type checking, and 24 Docker-backed database tests on Node `24.11.0`. `tiptap` passed its package and demo builds, lint, and 1,920 tests on that runtime. On Node `22.18.0`, `varlet` passed its bootstrap and 921 Istanbul coverage tests; its report-only lint step still reported the documented ambient `declare` error.
- `vinext` passed its build and checks on Node `24.11.0`. Its unit shard passed 4,615 tests with two skipped on Node `24.20.0`, with loopback requests excluded from the local proxy. The earlier `24.11.0` run exposed Node glob and React stream differences; tests that use `localhost` also failed until the proxy exclusion was set.
- `dify` passed its build and 41 tests in two files selected by the prescribed test filters. Its build skips type validation; the separate type check failed on the Node/browser matcher conflict below. `npmx.dev` passed formatting, type checking, and 1,652 Node tests. Its browser run passed 819 tests, failed 254, and skipped five; malformed translation-resource URLs reproduced the browser `define` issue below. Its lint step remains report-only.
- The WebDriverIO fixture passed with community provider `5.0.0-rc.1`. Preview passed with fake timers; locator clicks with real timers still reproduce the upstream failure below.

The stable release gate remains open. Of the 25 active ecosystem projects, 20 passed their prescribed workflows. Preflight blocked `decoders` on its Node `20.x` CI pin, `vue-mini` on its Node `25.9.0` runtime pin, and `yaak` on its Node `20` pin before changing project files. Their maintainers must choose supported test runtimes. `dify` and `npmx.dev` still fail required checks. External Linux and Windows jobs and a published Vite+ prerelease still need validation. Resolve the upstream failures below before the stable rollout.

## Alternatives

### Adopt all v5 defaults for existing projects

This keeps migrated configs smaller but makes an ordinary package update change mock history, project inheritance, plugin lifetimes, and locator matching at once. New projects still get these defaults. Existing projects should opt in after their current suite passes.

### Force v4 defaults inside `vp test`

This avoids config edits but makes Vite+ behavior differ from the Vitest v5 documentation and programmatic API. Explicit migration options are visible and removable.

### Mirror every v4 package and entry point

Several removed runner and suite APIs do not have a complete v5 target. Partial shims would fail later and with less useful errors. Keep only aliases with exact public replacements.

### Drop WebDriverIO support

The provider kept its package name and moved to community maintenance. An optional peer and compatibility shim preserve current Vite+ imports without coupling release versions. Drop the shim only if the community package cannot pass the v5 browser contract before the Vite+ stable release.

## Release-blocking questions

1. Does the final community `@vitest/browser-webdriverio` release pass the Vite+ browser suite with final Vitest v5? If not, document the provider exception and keep affected users on the v4-based Vite+ release.
2. Will the vendored Vite and Rolldown revisions move to Vitest v5 before the Vite+ release branch? If not, the temporary sync override and all affected upstream suites must land in the same change.
3. Does Preview support locator clicks with real timers? The final `5.0.0` packages currently fail because the Preview user-event adapter calls `vi.advanceTimersByTimeAsync()` without checking whether fake timers are enabled. A clean npm project with only upstream Vitest and Preview reproduces the failure. The `test_v5_preview` fixture records the failure separately from the passing fake-timer integration case. Do not treat that expected failure as a passed release gate.
4. Do browser globals receive decoded `define` values? Vitest `5.0.0` assigns JSON-encoded Vite values to browser globals. A string becomes `'"/messages"'` instead of `'/messages'`, and `'false'` remains a string. A clean upstream Preview project reproduces this without Vite+. `npmx.dev` requests quoted translation-resource URLs and fails browser assertions. The `upstream_browser_defines` fixture records the failure; an upstream fix or an approved compatibility patch is required.
5. Can Node jest-dom and browser matcher declarations coexist? With Vitest and Preview `5.0.0`, importing `vitest/browser` before the jest-dom augmentation rejects Node regex text assertions and CSS custom-property assertions. Reversing the imports passes the same type checks. The `test_v5_upstream_types` fixture verifies both orders with `@testing-library/jest-dom` versions `6.9.1` and `7.0.1`, without Vite+ installed. `dify` still fails its type check even with an explicit jest-dom type entry. Resolve the declaration conflict before enabling the upgrade for affected projects.
