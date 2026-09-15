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

## Node runtime

The migration upgrades incompatible runtime pins to the nearest supported minimum before installing dependencies. For example, `20.19.0` becomes `22.18.0`, `24.10.0` becomes `24.11.0`, and `25.9.0` becomes `26.0.0`. Supported pins stay unchanged. Unresolved selectors still need review.

Keep your library's public `engines.node` contract separate from its test runtime. The migration does not change that contract. An engine range with a supported minimum, such as `>=22.19.0`, needs no review.

The Node compatibility checks cover `.node-version`, `.nvmrc`, and the `engines.node`, `devEngines.runtime`, and `volta.node` declarations in `package.json`. Node versions in CI workflows, containers, and other files are outside the scope of these checks.

## Preserve existing behavior

For v4 configs, the migration adds compatibility settings where you omitted the corresponding option. Explicit settings take precedence.

| Setting                              | Reason to retain it during the upgrade                                 |
| ------------------------------------ | ---------------------------------------------------------------------- |
| `test.clearMocks: false`             | Retain mock history from setup files, `beforeAll`, and earlier tests.  |
| `test.sharedViteServer: false`       | Retain separate servers for inline projects.                           |
| Inline `extends: false`              | Retain v4's lack of root-config inheritance.                           |
| `browser.locators.exact: false`      | Retain partial and case-insensitive locator matching.                  |
| Reporter `{ stdout: true }`          | Retain stdout for JSON/JUnit reporters without an output file.         |
| Glob threshold `perFile: true`       | Retain enforcement inherited from `coverage.thresholds.perFile` in v4. |
| `fakeTimers.toNotFake: ['Temporal']` | Retain fake-timer behavior with a global Temporal polyfill.            |

New projects use v5 defaults. After the existing suite passes, remove compatibility settings one at a time to adopt those defaults.

The Vitest pass leaves configless projects without a config. In interactive mode, you can confirm a separate action to create a minimal compatibility config. A new config can change config discovery and project structure. Other migration steps, such as merging lint configuration, can create `vite.config.ts`; the v4 compatibility settings apply to that new config during the same migration.

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

The migration converts direct `bench` calls with literal names and inline, zero-argument callbacks. It preserves the workload callback and runs it through the v5 `bench` fixture inside a test. Simple `bench.skip`, `bench.only`, and `bench.todo` calls become the corresponding test modifiers. Calls inside ordinary `describe` and `suite` callbacks retain their surrounding scopes.

```ts
// Before
import { bench } from 'vitest';
bench('parse', () => JSON.parse('{"value":1}'));

// After
import { test as _test } from 'vitest';
_test('parse', async ({ bench: _bench }) => {
  await _bench('parse', () => JSON.parse('{"value":1}')).run();
});
```

Keep benchmark files matched by `benchmark.include`. The `vitest bench` and `vp test bench` commands remain supported. The migration does not remove them.

Review calls with benchmark options, dynamic names, callback references, or wrappers. Removed reporter settings and the `--compare` and `--outputJson` flags remain blockers. Baseline comparisons need per-benchmark result storage and an explicit comparison inside a test. See the [Vitest benchmark migration guide](https://vitest.dev/guide/migration/#benchmarking-api-rewrite) and [benchmarking guide](https://vitest.dev/guide/benchmarking) for replacements. Use `bench.compare()` if you need a comparison table across several workloads.

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
| `@vitest/ws-client`                      | Review the integration; upstream no longer adds v5 features.                |

Vite+ retains `vite-plus/test/coverage`, `/reporters`, `/environments`, `/snapshot`, and `/mocker` aliases through the Vite+ 1.x line. It does not provide partial runner or expect plugin shims.

The community maintains `@vitest/browser-webdriverio` on its own release schedule. Install a compatible peer and check browser behavior; do not force its version to match the official package patch version. Use `5.0.0` as the compatibility test baseline.

## Reports and screenshots

Update CI uploads, report consumers, and merge commands to use these paths. Keep old ignore entries while files remain in those directories. The migration adds `.vitest/` to `.gitignore` without deleting old entries.

| Output              | v4 default                    | v5 default                                 |
| ------------------- | ----------------------------- | ------------------------------------------ |
| Attachments         | `.vitest-attachements/`       | `.vitest/attachments/`                     |
| Failure screenshots | `__screenshots__/`            | `.vitest/attachments/failure-screenshots/` |
| Blob reports        | `.vitest-reports/blob-*.json` | `.vitest/blob/blob-*.json`                 |
| HTML                | `html/index.html`             | `.vitest/index.html`                       |
| JSON                | stdout                        | `.vitest/json/output.json`                 |
| JUnit               | stdout                        | `.vitest/junit/output.xml`                 |

HTML reporter options now use `outputDir` instead of the file-valued `outputFile`. The migration converts an `index.html` path to its parent directory and reports custom filenames for review. Other reporters retain `outputFile` support.

Copy a custom `browser.screenshotDirectory` to `browser.expect.toMatchScreenshot.screenshotDirectory`. Review reference images before moving or regenerating them; v4 could save them in an unintended location. The default reference screenshot directory remains `__screenshots__`.

## Resolve migration findings

Each Vitest v5 `BLOCK` or `REVIEW` item includes a `Docs` link. Follow the link for the relevant API change or migration rule. A blocker stops dependency updates; a review item requires you to check the result but does not stop safe edits.

For dynamic configs or shared files, determine the effective Vitest project settings before editing. Resolve config functions, spreads, and external inheritance, or make the compatibility changes by hand. Confirm which runner owns unimported globals in a mixed-runner project.

For parser failures or overlapping edits, keep the original file and apply the documented change by hand. Run `vp migrate` again after resolving the issue. A successful exit can still include review items; keep that report until you finish the manual changes.

## Review checklist

Use the file locations in the migration report to review these changes. Run browser and coverage suites as well as Node tests.

For jest-dom matcher types, load `@testing-library/jest-dom/vitest` in `compilerOptions.types` or import it from a TypeScript setup file that your `tsconfig.json` includes. The root `@testing-library/jest-dom` type entry augments Jest. A JavaScript setup file excluded by `allowJs: false` does not load the Vitest augmentation for type checking. Review shared Jest/Vitest configs before changing their type entries.

With `vitest@5.0.1`, loading browser declarations first can also make TypeScript reject valid Node jest-dom assertions: `toHaveTextContent(/pattern/)` and CSS custom properties in `toHaveStyle()`. We reproduced this conflict with upstream imports and with Vite+ imports. Both `@testing-library/jest-dom@6.9.1` and `7.0.1` reproduce it. Check the declarations loaded by each test `tsconfig.json`; adding the jest-dom type entry alone might not resolve the conflict. Keep the affected project on the v4-based release until its Node and browser matcher types pass separate checks.

| Area                     | Required review                                                                                                                                                                            |
| ------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Test names and snapshots | Full names use `>` separators. Check `-t` patterns, `test.each`/`test.for` titles, and `pretty-format` output.                                                                             |
| Hoisted mocks            | Move nested `vi.mock`, `vi.unmock`, and `vi.hoisted` calls to the top level after checking captured variables.                                                                             |
| Browser automocks        | Factory-free mocks retain mock defaults. Choose `{ spy: true }` if you need real implementations.                                                                                          |
| Constructor mocks        | Check prototypes, methods, and `instanceof` for `vi.fn`, `vi.spyOn`, and `mockImplementation`.                                                                                             |
| Benchmarks               | Review benchmark calls that the migration could not convert, comparison groups, and removed reporting options. Keep supported options such as `benchmark.enabled` and `benchmark.include`. |
| Async assertions         | Await or return `resolves`, `rejects`, file snapshots, polls, and browser assertions. Review `expect.poll` timeouts without increasing them to hide failures.                              |
| Matcher types            | Update old `Assertion<T>`, `Matchers<T>`, and `jest.Matchers` declarations for return and received type parameters.                                                                        |
| Browser commands         | Accept `SerializedLocator` objects instead of selector strings.                                                                                                                            |
| Browser UI               | Use Vitest's printed URLs, including UI tokens and orchestrator `sessionId`. Check headed/headless screenshots at fixed viewports.                                                         |
| Project configuration    | Check dynamic projects, root-config merges, nested projects, and hooks that assume one server or config execution per project.                                                             |
| Test collection          | Review dynamic command wrappers and programmatic `collect()` calls whose static-parse options need manual edits.                                                                           |
| Coverage                 | Compare v4/v5 resolved file sets for configured `coverage.include` and `coverage.exclude`. Relative-path matching can change the measured set.                                             |
| Worker IDs               | Review arithmetic and indexing based on `VITEST_POOL_ID` or `VITEST_WORKER_ID`; IDs start at 1.                                                                                            |
| Custom environments      | Restore `populateGlobal().originals` entries with `Object.defineProperty`; the map contains descriptors.                                                                                   |
| DOM globals              | Check assignments in jsdom/happy-dom tests; global assignments also update the window.                                                                                                     |
| Temporal                 | Review scopes that use Temporal and `vi.setSystemTime()`. `toNotFake` does not preserve behavior without fake timers.                                                                      |
| Removed APIs             | Replace unsupported runner and internal imports before updating packages. Type-only references still require review and type checks.                                                       |

## Run below a config directory

Vitest v5 no longer searches parent directories for a config. From a subdirectory, pass both the parent config and the intended test directory:

```bash
vp test --config ../vite.config.ts --dir .
```

See the [upstream Vitest 5 migration guide](https://vitest.dev/guide/migration/) for API examples and the new benchmark design.
