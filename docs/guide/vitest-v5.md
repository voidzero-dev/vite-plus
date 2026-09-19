# Upgrade to Vitest 5

Vite+ bundles Vitest `5.0.1`. Use Node `^22.18.0 || ^24.11.0 || >=26.0.0` for the CLI and tests.

## Before you migrate

Run `vp migrate` from the workspace root before updating dependencies by hand. Keep the original lockfile and installed packages available so the migration can identify the source Vitest version, including the runner inside an older `vite-plus` installation.

Upgrade projects older than Vitest 4 to v4 before running this migration. If the original version cannot be determined, restore and install the original lockfile, then rerun `vp migrate`.

```bash
vp migrate
vp install
vp check
vp test
```

Read the file-specific review report before committing the result. The migration stops before dependency updates for conflicting `test.api` and `test.browser.api` settings or removed runtime APIs without a supported replacement. Resolve these blockers and run the command again. You can finish safe edits with a successful exit status while review items remain.

You can keep equivalent static `test.api` and `test.browser.api` values, including objects with different property order or quote styles. The migration removes the redundant `browser.api`. Choose one configuration for conflicting values or expressions that need evaluation; v5 uses one API server.

## Node runtime

The migration upgrades incompatible runtime pins to the nearest supported minimum before installing dependencies. For example, `20.19.0` becomes `22.18.0`, `24.10.0` becomes `24.11.0`, and `25.9.0` becomes `26.0.0`. Supported pins stay unchanged. Unresolved selectors still need review.

Keep your library's public `engines.node` contract separate from its test runtime. The migration does not change that contract. You do not need to review an engine range with a supported minimum, such as `>=22.19.0`, or a whole supported major, such as `24.x`. Use a concrete runtime pin at or above the supported minimum for that major.

The Node compatibility checks cover `.node-version`, `.nvmrc`, and the `engines.node`, `devEngines.runtime`, and `volta.node` declarations in `package.json`. Node versions in CI workflows, containers, and other files are outside the scope of these checks.

## Preserve existing behavior

For v4 configs, the migration adds compatibility settings where you omitted the corresponding option. Explicit settings take precedence. Beside each added setting, you get a comment with the reason, guidance for adopting v5 behavior, and a link to the Vitest migration guide:

```ts
test: {
  // Vitest v4 compatibility: preserve mock call history.
  // Remove after tests no longer rely on calls from setup or earlier tests.
  // https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default
  clearMocks: false,
}
```

| Added setting                                                                                                               | Behavior to check before removing it                                                                                                    |
| --------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| [`test.clearMocks: false`](https://vitest.dev/guide/migration/#clearmocks-is-enabled-by-default)                            | Check assertions that depend on mock calls from setup, `beforeAll`, or earlier tests. V5 clears that history before each test.          |
| [`test.sharedViteServer: false`](https://vitest.dev/guide/migration/#inline-projects-share-the-vite-server-by-default)      | Check plugins and config hooks with a shared server. V5 initializes them once for eligible inline projects.                             |
| [Inline `extends: false`](https://vitest.dev/guide/migration/#inline-projects-inherit-the-root-config-by-default)           | Check inherited root options, plugins, and setup files. V5 merges arrays, so avoid duplicate setup.                                     |
| [`browser.locators.exact: false`](https://vitest.dev/guide/migration/#locators-are-strict-by-default)                       | Update locators for full, case-sensitive matches, or opt out on individual locators.                                                    |
| [Glob threshold `perFile: true`](https://vitest.dev/guide/migration/#glob-coverage-thresholds-no-longer-inherit-perfile)    | Keep for per-file coverage enforcement. Remove to check matching files as a group.                                                      |
| [`fakeTimers.toNotFake: ['Temporal']`](https://vitest.dev/guide/migration/#fake-timers-and-setsystemtime-now-mock-temporal) | Check tests that use the global Temporal polyfill with fake timers. Remove only `Temporal` to use mocked time; retain other exclusions. |

New projects use v5 defaults. To adopt those defaults in an existing project, change one setting at a time and run the affected tests. Check CI and report consumers for output changes. Remove the accompanying comment after you accept the new behavior, or replace it with your project's reason for keeping the setting. These options remain supported in v5; you do not need to remove them all.

The migration leaves your existing settings and comments untouched. It does not add comments to settings from an earlier migration, since it cannot distinguish those settings from your own choices. Repeated runs do not duplicate comments or restore comments you removed.

If your project has no test config, you can use the v5 defaults without a review prompt or a new compatibility config. Other migration steps, such as merging lint configuration, can create `vite.config.ts`; the v4 compatibility settings apply to that new config during the same migration.

After upgrading to v5, you can rerun `vp migrate` without reapplying v4 compatibility defaults, including in projects without a config. Finish the dependency installation before rerunning migration.

Resolve or save the first run's review report before discarding the original dependencies. Later runs use the current Vitest version and files, so they might not repeat reviews that depended on the original v4 behavior.

## Source changes

The migration uses syntax and import bindings for targeted edits. You get a review item for dynamic options, ambiguous wrappers, or a callback whose contract cannot accept `async`.

```ts
// Before
test.sequential('result', () => {
  expect(result).resolves.toBe(42);
});

// After
test('result', { concurrent: false }, async () => {
  await expect(result).resolves.toBe(42);
});
```

In browser tests, use `toMatchTextContent` for v4 string and regular-expression assertions. The v4 string matcher accepted partial text, so identical text in a fixture does not prove that you intended v5's exact match. Choose `toHaveTextContent` yourself if you want the new semantics.

Keep Node `@testing-library/jest-dom` assertions unchanged. In a mixed Node/browser package, the migration checks static project membership before renaming a matcher. You must review shared files, dynamic configs, and command-line browser overrides. Calls through `expect.element()` identify browser assertions without that ambiguity.

Await `render` from `vitest-browser-vue` and `vitest-browser-svelte` before querying its result. Replace `toThrow('')` and `toThrowError('')` with `/^$/` to keep assertions for an empty message.

For programmatic config loading, replace `{ viteConfig, vitestConfig }` destructuring with the return value from `resolveConfig()` and its `.test` property. Direct migrated `vitest list` commands get `--no-static-parse`; supported `Vitest.collect(filters, options)` calls get `{ staticParse: false }`. Keep an explicit static-parse choice if you want v5's static collection.

## Benchmarks

Use direct `bench` calls with inline, zero-argument callbacks or references to unchanged local functions. You can use dynamic names; the migration captures them once during registration. It preserves the workload callback and runs it through the v5 `bench` fixture inside a test. Simple `bench.skip`, `bench.only`, and `bench.todo` calls become the corresponding test modifiers. Calls inside ordinary `describe` and `suite` callbacks retain their surrounding scopes.

```ts
// Before
import { bench } from 'vitest';
bench('parse', () => JSON.parse('{"value":1}'));

// After
import { test } from 'vitest';
test('parse', async ({ bench }) => {
  await bench('parse', () => JSON.parse('{"value":1}')).run();
});
```

Keep benchmark files matched by `benchmark.include`. The `vitest bench` and `vp test bench` commands remain supported. The migration does not remove them.

You can migrate built-in `benchmark.reporters` settings (`default` and `verbose`) and literal `benchmark.outputFile` or `benchmark.outputJson` paths to top-level reporters. Existing reporter destinations take precedence over a proposed move: resolve conflicting paths, terminal reporter choices, and JSON stdout consumers by hand. Project-specific settings, config merges, and custom or dynamic reporters also need review. In v5, regular tests and benchmarks share reporters.

For a literal command such as `vitest bench --outputJson=bench.json`, the migration selects the JSON reporter and keeps the file path. The v5 JSON report contains test results with per-test benchmarks. If you consume these reports outside Vitest, update readers that expect the v4 format.

Review calls with benchmark options, imported or reassigned callbacks, callback parameters, or wrappers. Baseline `compare` settings and `--compare` flags remain blockers. Set up per-benchmark result storage and an explicit comparison inside a test. See the [Vitest benchmark migration guide](https://vitest.dev/guide/migration/#benchmarking-api-rewrite) and [benchmarking guide](https://vitest.dev/guide/benchmarking) for replacements. Use `bench.compare()` if you need a comparison table across several workloads.

## Entry points and packages

Use `vite-plus/test` for assertions and supported runner APIs. Keep coverage providers and `@vitest/ui` at the bundled runner's exact version, `5.0.1`. Keep `@vitest/web-worker` aligned when your project uses it.

| v4 import                                | v5 import or action                                                         |
| ---------------------------------------- | --------------------------------------------------------------------------- |
| `vitest/coverage`, `vitest/reporters`    | `vite-plus/test/node`                                                       |
| `vitest/environments`, `vitest/snapshot` | `vite-plus/test/runtime`                                                    |
| `vitest/mocker`                          | `vite-plus/test/mocker`                                                     |
| `vitest/runners`, `vitest/suite`         | Review `TestRunner` and its static methods from `vite-plus/test`.           |
| Root-compatible `@vitest/expect` symbols | `vite-plus/test`; review unsupported helpers.                               |
| `@vitest/runner`                         | Migrate supported symbols to the root API and review the remaining imports. |
| `vitest/internal/module-runner`          | Redesign the integration; no public replacement exists.                     |
| `@vitest/ws-client`                      | Retain if needed; upstream no longer adds v5 features.                      |

Vite+ retains `vite-plus/test/coverage`, `/reporters`, `/environments`, `/snapshot`, and `/mocker` aliases through the Vite+ 1.x line. It does not provide partial runner or expect plugin shims.

## Community WebDriverIO provider

Vite+ 1.0 removes the `vite-plus/test/browser-webdriverio` exports. Use `@vitest/browser-webdriverio` and choose a provider version and framework peers that support your tests. The community maintains this package on its own release schedule.

`vp migrate` changes legacy provider imports to `@vitest/browser-webdriverio` and ensures a provider version of at least `5.0.0`. It adds `^5.0.0` when the provider is missing, upgrades older versions, and narrows ranges that still allow v4. It also updates referenced catalog entries and removes overrides that force an older provider. Versions and ranges already above this minimum stay unchanged. The migration ensures the required `webdriverio` peer is installed without upgrading an existing framework version.

You manage provider upgrades after this migration; Vite+ does not synchronize community releases with Vitest. Existing imports from the community package stay unchanged. For custom dependencies that the migrator cannot verify, resolve the diagnostic before retrying migration. See the [upstream package migration guide](https://vitest.dev/guide/migration/#package-migration).

Legacy Vite+ WebDriverIO `/context` imports move to `vite-plus/test/browser/context`. Keep runtime browser APIs on this shared entry; the community provider's `/context` entry contains types only.

The optional peer declaration still follows Vitest's dependency metadata. It does not provide a Vite+ export or synchronize provider releases.

## Reports and screenshots

The migration does not edit or report changes that affect only title formatting, report output, artifact paths, or dependency deprecation. It leaves regular reporter settings unchanged and does not add `stdout: true`. Removed benchmark APIs and flags still need migration so commands can run.

If you consume generated reports or artifacts outside Vitest, check these paths. Keep old ignore entries while files remain in those directories. The migration adds `.vitest/` to `.gitignore` without deleting old entries.

| Output              | v4 default                    | v5 default                                 |
| ------------------- | ----------------------------- | ------------------------------------------ |
| Attachments         | `.vitest-attachements/`       | `.vitest/attachments/`                     |
| Failure screenshots | `__screenshots__/`            | `.vitest/attachments/failure-screenshots/` |
| Blob reports        | `.vitest-reports/blob-*.json` | `.vitest/blob/blob-*.json`                 |
| HTML                | `html/index.html`             | `.vitest/index.html`                       |
| JSON                | stdout                        | `.vitest/json/output.json`                 |
| JUnit               | stdout                        | `.vitest/junit/output.xml`                 |

HTML reporter options now use `outputDir` instead of the file-valued `outputFile`. Set `outputDir` yourself if you need a custom destination. Other reporters retain `outputFile` support. See the [upstream report changes](https://vitest.dev/guide/migration/#generated-reports-and-artifacts-use-the-vitest-directory).

Copy a custom `browser.screenshotDirectory` to `browser.expect.toMatchScreenshot.screenshotDirectory`. Review reference images before moving or regenerating them; v4 could save them in an unintended location. The default reference screenshot directory remains `__screenshots__`.

## Resolve migration findings

Each Vitest v5 `BLOCK` or `REVIEW` item includes a `Docs` link. Follow the link for the relevant API change or migration rule. A blocker stops dependency updates; a review item requires you to check the result but does not stop safe edits.

For dynamic configs or shared files, determine the effective Vitest project settings before editing. Resolve config functions, spreads, and external inheritance, or make the compatibility changes by hand. Confirm which runner owns unimported globals in a mixed-runner project.

For parser failures or overlapping edits, keep the original file and apply the documented change by hand. Run `vp migrate` again after resolving the issue. A successful exit can still include review items; keep that report until you finish the manual changes.

## Review checklist

Use the file locations in the migration report to review these changes. Run browser and coverage suites as well as Node tests.

For jest-dom matcher types, load `@testing-library/jest-dom/vitest` in `compilerOptions.types` or import it from a TypeScript setup file that your `tsconfig.json` includes. The root `@testing-library/jest-dom` type entry augments Jest. A JavaScript setup file excluded by `allowJs: false` does not load the Vitest augmentation for type checking. Review shared Jest/Vitest configs before changing their type entries.

With `vitest@5.0.1`, loading browser declarations first can also make TypeScript reject valid Node jest-dom assertions: `toHaveTextContent(/pattern/)` and CSS custom properties in `toHaveStyle()`. We reproduced this conflict with upstream imports and with Vite+ imports. Both `@testing-library/jest-dom@6.9.1` and `7.0.1` reproduce it. Check the declarations loaded by each test `tsconfig.json`; adding the jest-dom type entry alone might not resolve the conflict. Keep the affected project on the v4-based release until its Node and browser matcher types pass separate checks.

| Area                  | Required review                                                                                                                                                                            |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Test selection        | Full names use `>` separators. Check name filters that span suite boundaries; plain single-segment filters do not need review.                                                             |
| Hoisted mocks         | Move nested `vi.mock`, `vi.unmock`, and `vi.hoisted` calls to the top level after checking captured variables.                                                                             |
| Browser automocks     | Factory-free mocks retain mock defaults. Choose `{ spy: true }` if you need real implementations.                                                                                          |
| Constructor mocks     | Check prototypes, methods, and `instanceof` for `vi.fn`, `vi.spyOn`, and `mockImplementation`.                                                                                             |
| Benchmarks            | Review benchmark calls that the migration could not convert, comparison groups, and removed reporting options. Keep supported options such as `benchmark.enabled` and `benchmark.include`. |
| Async assertions      | Await or return `resolves`, `rejects`, file snapshots, polls, and browser assertions. Review `expect.poll` timeouts without increasing them to hide failures.                              |
| Matcher types         | Update old `Assertion<T>`, `Matchers<T>`, and `jest.Matchers` declarations for return and received type parameters.                                                                        |
| Browser commands      | Accept `SerializedLocator` objects instead of selector strings.                                                                                                                            |
| Browser UI            | Use Vitest's printed URLs, including UI tokens and orchestrator `sessionId`. Check headed/headless screenshots at fixed viewports.                                                         |
| Project configuration | Check dynamic projects, root-config merges, nested projects, and hooks that assume one server or config execution per project.                                                             |
| Test collection       | Review dynamic command wrappers and programmatic `collect()` calls whose static-parse options need manual edits.                                                                           |
| Coverage              | Compare v4/v5 resolved file sets for configured `coverage.include` and `coverage.exclude`. Relative-path matching can change the measured set.                                             |
| Worker IDs            | Review arithmetic and indexing based on `VITEST_POOL_ID` or `VITEST_WORKER_ID`; IDs start at 1.                                                                                            |
| Custom environments   | Restore `populateGlobal().originals` entries with `Object.defineProperty`; the map contains descriptors.                                                                                   |
| DOM globals           | Check assignments in jsdom/happy-dom tests; global assignments also update the window.                                                                                                     |
| Temporal              | Review scopes that use Temporal and `vi.setSystemTime()`. `toNotFake` does not preserve behavior without fake timers.                                                                      |
| Removed APIs          | Replace unsupported runner and internal imports before updating packages. Type-only references still require review and type checks.                                                       |

## Run below a config directory

Vitest v5 no longer searches parent directories for a config. From a subdirectory, pass both the parent config and the intended test directory:

```bash
vp test --config ../vite.config.ts --dir .
```

See the [upstream Vitest 5 migration guide](https://vitest.dev/guide/migration/) for API examples and the new benchmark design.
