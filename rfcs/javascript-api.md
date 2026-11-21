# RFC: Vite+ JavaScript API

- Status: Proposed
- Tracking: 2026 Q4 plan, [#3277](https://github.com/voidzero-dev/vite-plus/issues/3277), item "P1: Define the Vite+ JavaScript API"
- Related: [check-command.md](./check-command.md), [config-and-staged-commands.md](./config-and-staged-commands.md), [package-manager-detection.md](./package-manager-detection.md), [packages/cli/BUNDLING.md](../packages/cli/BUNDLING.md), [docs/guide/run.md](../docs/guide/run.md)

## Summary

The `vite-plus` package already exposes the JavaScript APIs of the tools it bundles. `vite-plus` re-exports Vite, `vite-plus/test/node` re-exports the Vitest Node API, `vite-plus/pack` re-exports tsdown, and `vite-plus/fmt` re-exports Oxfmt.

The features that Vite+ owns have no JavaScript API. These are the task runner (`vp run`), the composite check (`vp check`), and workspace and package-manager detection. A script that needs them starts a `vp` process, then reads an exit code or parses text.

This RFC defines JavaScript APIs for the Vite+-owned features. It selects a first scope, gives signatures and examples, and lists the work for later phases.

| Decision             | Items                                                                                                                            |
| -------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Phase 1 (this RFC)   | `vite-plus/workspace`: `findWorkspace()`. `vite-plus/run`: `getTaskGraph()`, `planRun()`, `run()`. `vite-plus/check`: `check()`. |
| Unchanged            | `vite-plus/lint` and `vite-plus/fmt` keep the Oxlint and Oxfmt APIs. Only the import path differs from upstream.                 |
| Later phases         | `staged()`, `cleanCache()`, package-manager commands (`install()`, `add()`, `remove()`), `create()`, `migrate()`.                |
| Non-goals            | Global CLI features (`vp env`, `vp upgrade`, `vp implode`). New wrappers around the Vite, Vitest, tsdown, Oxlint, or Oxfmt APIs. |
| Stability in phase 1 | All phase 1 APIs are experimental. The task graph and run plan documents already carry their own format version.                 |

## Lint and Fmt: What Differs From Oxlint and Oxfmt

A reviewer of the plan asked: "What should be different from the Oxlint/Oxfmt API here, and what is the goal for end users?"

**Nothing differs in the API.** `vite-plus/lint` and `vite-plus/fmt` re-export the bundled Oxlint and Oxfmt packages. The functions, signatures, and types are the upstream ones. Only the import specifier changes.

**The goal for end users is one dependency and one version.** An import through `vite-plus` always resolves to the Oxlint and Oxfmt versions that `vp lint` and `vp fmt` run. The project does not pin a second copy that can drift. The import also resolves under pnpm's strict layout from any package that depends on `vite-plus`. [docs/guide/lint.md](../docs/guide/lint.md) already states this for the plugin entries.

This RFC does not design a new lint or format API. The design work below is for the features that Vite+ owns.

### Current status

Bundled versions come from `pnpm-workspace.yaml`: oxlint 1.94.0 and oxfmt 0.79.0.

| Entry                        | Source file                            | Runtime exports today                              | Upstream runtime exports                         |
| ---------------------------- | -------------------------------------- | -------------------------------------------------- | ------------------------------------------------ |
| `vite-plus/fmt`              | `packages/cli/src/fmt.ts`              | `format`, plus all types                           | `oxfmt`: `format`, `defineConfig`, `jsTextToDoc` |
| `vite-plus/lint`             | `packages/cli/src/lint.ts`             | None. Types only.                                  | `oxlint`: `defineConfig`                         |
| `vite-plus/lint/plugins`     | `packages/cli/src/lint-plugins.ts`     | `definePlugin`, `defineRule`, `eslintCompatPlugin` | `@oxlint/plugins`: the same three                |
| `vite-plus/lint/plugins-dev` | `packages/cli/src/lint-plugins-dev.ts` | `RuleTester`                                       | `oxlint/plugins-dev`: `RuleTester`               |

Facts that follow from the table:

- Oxlint 1.94.0 has no programmatic lint function. Its main entry exports `defineConfig` and types. `vite-plus/lint` therefore has nothing more to re-export today. When Oxlint adds a lint function, `vite-plus/lint` re-exports it unchanged.
- `vite-plus/lint` omits `defineConfig` on purpose. The comment in `packages/cli/src/lint.ts` says that Vite+ users should use `defineConfig()` from `vite-plus`. Lint settings go in the `lint` block of `vite.config.ts`.
- `vite-plus/fmt` exposes `format(fileName, sourceText, options?)`. It is the one function in Oxfmt's main entry that formats source text and returns the result. The entry omits `defineConfig` and `jsTextToDoc`.

### What is missing

| Gap                                                                                                        | Proposed action                                                                                   |
| ---------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------- |
| `docs/guide/` and `docs/config/` do not mention `vite-plus/fmt`.                                           | Document `format` from `vite-plus/fmt` in the format guide.                                       |
| `vp migrate` does not rewrite `oxfmt` imports. The `vite-plus/prefer-vite-plus-imports` rule ignores them. | Rewrite `oxfmt` to `vite-plus/fmt` in both. See open question 7.                                  |
| `vp migrate` leaves config-surface `oxlint` imports alone, such as `OxlintConfig` and `OxlintOverride`.    | Rewrite type imports to `vite-plus/lint`. See open question 7.                                    |
| `vite-plus/fmt` omits `defineConfig` and `jsTextToDoc`.                                                    | Keep `defineConfig` out, for the same reason as lint. Decide on `jsTextToDoc` in open question 6. |

These are import-path and documentation tasks. They need no API design.

## Goals for End Users

A JavaScript API serves callers that are programs. The CLI serves them poorly in four ways:

- **Location.** A script must find the `vp` binary. Without the global CLI, that means `pnpm exec vp`, `npx vp`, or a path into `node_modules/.bin`. An import of `vite-plus/run` uses normal module resolution.
- **Results.** `vp run build` and `vp check` report through an exit code and text for people. The caller cannot tell which task failed, which step was a cache hit, or which file has a lint error without parsing that text.
- **Options.** Flags are strings. A typo in a flag fails at run time. Typed options fail in the editor.
- **Progress and control.** A child process gives the caller a byte stream and a kill signal. It gives no task events and no cancellation that the task runner understands.

| User group                      | Workflow that the API makes possible                                                                                                                                          | What the CLI offers today                                                                                         |
| ------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------- |
| Script authors                  | A release script builds selected packages, reads which tasks failed, and publishes only when the result is `ok`.                                                              | `vp run` in a child process. Exit code and text only.                                                             |
| CI pipelines                    | A pipeline step reads the run plan, skips a job when every step is a cache hit, and turns check diagnostics into annotations.                                                 | `vp run --dry-run --json` exists. `vp check` has no structured output. `vp lint --format=json` covers lint alone. |
| Framework and tool integrations | A framework CLI runs the project's tasks and checks without knowing the package manager or the binary path. It reads the detected package manager to print the right command. | No public detection API. The NAPI function `detectWorkspace` is internal.                                         |
| Editor extensions               | A task panel lists every package and task with its dependencies. A check command maps diagnostics to file positions.                                                          | `vp run --json` exists. Diagnostics come from the Oxc editor extension or from text parsing.                      |
| AI agents                       | An agent runs `check()`, reads `file`, `line`, `code`, and `help` for each diagnostic, fixes the code, and runs again. It reads the plan before it runs an expensive task.    | Text output. `vp run --json` and `--dry-run --json` for the task runner only.                                     |
| Monorepo tools                  | A dashboard or affected-package tool reads the package graph and the task graph as typed data.                                                                                | `vp run --json` in a child process.                                                                               |

The task graph and the run plan already have JSON output. For those two, the API adds types, removes the child process, and removes the binary lookup. For `run()` and `check()`, the API adds results that do not exist today.

## Current State

### Existing public entries

Checked against the `exports` map in `packages/cli/package.json`.

| Entry                                                                                     | Content                                                                                                             | Owner of the API |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- | ---------------- |
| `vite-plus`                                                                               | All of `vite`, plus `defineConfig`, `defineProject`, `lazyPlugins`, `resolveConfigFromFile`, Vitest config defaults | Vite, Vite+      |
| `vite-plus/test/node`                                                                     | Vitest Node API                                                                                                     | Vitest           |
| `vite-plus/test/*`                                                                        | `vite-plus/test` and its subpaths: the Vitest exports, generated by `syncTestPackageExports()`                      | Vitest           |
| `vite-plus/pack`                                                                          | `vite/pack`, which is tsdown, plus the `PackUserConfig` type                                                        | tsdown           |
| `vite-plus/fmt`, `vite-plus/lint`, `vite-plus/lint/plugins`, `vite-plus/lint/plugins-dev` | See the previous section                                                                                            | Oxc              |
| `vite-plus/toolchain`                                                                     | The toolchain manifest, generated by `syncToolchainExports()` in `packages/cli/build.ts`                            | Vite+            |
| `vite-plus/versions`                                                                      | Bundled tool versions, derived from the same manifest                                                               | Vite+            |
| `vite-plus/register`                                                                      | Node.js preload for TypeScript scripts                                                                              | Vite+            |

The NAPI binding in `packages/cli/binding/` is not in the `exports` map.

### Vite+-owned features without a public API

| Feature                                 | CLI                                             | Implementation today                                                                                                                                              |
| --------------------------------------- | ----------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Task runner                             | `vp run`, `vpr`                                 | Vite Task `Session`, from the `vt` git dependency. `execute_vite_task_command` in `packages/cli/binding/src/cli/mod.rs` calls it.                                 |
| Composite check                         | `vp check`                                      | `execute_check` in `packages/cli/binding/src/check/mod.rs`. It runs oxfmt and oxlint as child processes and parses their text output.                             |
| Workspace and package-manager detection | No command of its own                           | `vt_workspace::find_workspace_root` and `vp_pm_cli::get_package_manager_type_and_version`. The NAPI functions `detectWorkspace` and `findMonorepoRoot` wrap them. |
| Staged files                            | `vp staged`                                     | `packages/cli/src/staged/bin.ts` calls the lint-staged programmatic API with the `staged` config block.                                                           |
| Task cache                              | `vp cache clean`                                | Vite Task `CacheSubcommand::Clean`.                                                                                                                               |
| Package management                      | `vp install`, `vp add`, `vp remove`, and others | `vp_pm_cli::dispatch_for_local_cli`. It forwards to the detected package manager.                                                                                 |
| Project creation and migration          | `vp create`, `vp migrate`                       | `packages/cli/src/create/bin.ts` and `packages/cli/src/migration/bin.ts`. Both are CLI scripts with prompts and `process.exit` calls.                             |
| Runtime management                      | `vp env`                                        | Global CLI only. [docs/guide/env.md](../docs/guide/env.md) states that the local package does not include it.                                                     |

### How commands run today

`packages/cli/src/bin.ts` handles `create`, `migrate`, `sync-versions`, `config`, `hooks`, `staged`, and `--version` in JavaScript. It sends every other command to the NAPI function `run(options)` in `packages/cli/binding/src/lib.rs`.

That function takes the CLI arguments and a set of JavaScript callbacks. The callbacks resolve tool binaries (`lint`, `fmt`, `vite`, `test`, `pack`, `doc`) and load `vite.config.ts` (`resolveUniversalViteConfig`). It returns an exit code. Rust code and the child processes write straight to the stdout and stderr of the process.

Three properties of this path matter for an API:

- Vite Task prints its own errors and returns only an exit status. The comment in `execute_vite_task_command` says so.
- The CLI path changes process-wide state. `prepend_tools_to_path_env` and `stamp_package_manager_lifecycle_env` call `std::env::set_var`. `bin.ts` calls `process.chdir` for `-C`.
- The binding's `module_init` calls `ensure_blocking_stdio()`, installs a panic hook, and starts tracing when the module loads.

## Design Principles

1. **One subpath per feature, named after the command.** `vp run` maps to `vite-plus/run`. `vp check` maps to `vite-plus/check`. This matches `vite-plus/lint`, `vite-plus/fmt`, `vite-plus/pack`, and `vite-plus/test`. The root entry stays as it is. It re-exports all of `vite`, so a new root export can collide with a future Vite export, and every root import loads Vite.
2. **The CLI and the API share one implementation.** A command is three parts: parse arguments, do the work, render the result. The API is the middle part. The CLI calls the same code and adds the other two. A behavior fix lands once.
3. **Return data. Do not print.** An API call writes nothing to stdout or stderr on its own behalf. It prints no header, no summary, and no error text. It never calls `process.exit` and never sets `process.exitCode`. It never prompts.
4. **A failed task is a result, not an exception.** A non-zero task exit or a lint error resolves the promise with `ok: false`. The promise rejects only when the call cannot do its job: no workspace, a config file that does not load, an unknown task, an invalid option.
5. **No process-wide state.** A call takes `cwd` and `env` as options. It does not call `process.chdir` and does not change `process.env`.
6. **Options mirror CLI flags.** Each option has the camelCase name of its flag and the same default. A negative flag becomes a boolean: `--no-fmt` is `fmt: false`. A reader of [docs/guide/run.md](../docs/guide/run.md) can predict the option name.
7. **Reuse versioned documents.** Where the CLI already prints a JSON document with a `version` field, the API returns that document as an object. Vite Task owns the format.
8. **The local package is enough.** The API lives in `vite-plus` and must not need the global `vp` CLI. This follows the boundary rule in the root `AGENTS.md`.
9. **Upstream APIs pass through unchanged.** Vite+ does not wrap or rename the Vite, Vitest, tsdown, Oxlint, or Oxfmt APIs.
10. **ESM and types.** The new entries are ESM with `.d.ts` files, like `./fmt`, `./lint`, and `./pack`. The supported Node.js range is the `engines` field of `packages/cli/package.json`.
11. **Stability levels.** An API is `experimental` or `stable`. An experimental API can change in a minor release with a changelog entry. Its JSDoc carries `@experimental`. Phase 1 ships everything as experimental.

## Candidate Features

Value is the number of user groups that need the feature. Cost is the implementation work. Risk is the chance that the API shape must change after release.

| Candidate                                  | User value                                              | Cost                                                                                     | Stability risk                                           | Decision  |
| ------------------------------------------ | ------------------------------------------------------- | ---------------------------------------------------------------------------------------- | -------------------------------------------------------- | --------- |
| Workspace and package-manager detection    | High. Every integration needs it first.                 | Low. NAPI functions exist.                                                               | Low. The detection order is documented.                  | Phase 1   |
| Task graph (`vp run --json`)               | High for editors, agents, monorepo tools.               | Low. `Session::task_graph_json()` is public in Vite Task.                                | Low. The document has `version: 1`.                      | Phase 1   |
| Run plan (`vp run --dry-run --json`)       | High for CI and agents.                                 | Low to medium. Vite Task must return the document as a value.                            | Low. The document has `version: 1`.                      | Phase 1   |
| Run tasks (`vp run`)                       | Highest. It is the main Vite+-owned workflow.           | Medium to high. Vite Task needs a public reporter, a result document, and a cancel hook. | Medium. The result format is new.                        | Phase 1   |
| Check (`vp check`)                         | High for CI, agents, editors.                           | Medium. `execute_check` must separate work from printing.                                | Medium. Lint diagnostics follow Oxlint's JSON reporter.  | Phase 1   |
| Staged files (`vp staged`)                 | Low. Git hooks call the CLI.                            | Low. It wraps the lint-staged API.                                                       | Low.                                                     | Phase 2   |
| Cache clean (`vp cache clean`)             | Low to medium for scripts.                              | Low.                                                                                     | Low.                                                     | Phase 2   |
| Package management (`install`, `add`, ...) | Medium for scripts and scaffolding tools.               | Medium. Many flags differ by package manager.                                            | High. Output keeps each package manager's native format. | Phase 2   |
| Create and migrate                         | Medium for scaffolding tools and agents.                | High. About 1,450 and 1,840 lines of CLI script with prompts.                            | High. Options change often.                              | Phase 3   |
| Runtime management (`vp env`)              | Medium.                                                 | Not applicable.                                                                          | Not applicable.                                          | Non-goal  |
| Lint and format                            | Covered upstream.                                       | None.                                                                                    | Owned by Oxc.                                            | No change |
| Build, dev, preview, test, pack            | Covered upstream.                                       | None.                                                                                    | Owned by Vite, Vitest, tsdown.                           | No change |
| Toolchain and versions                     | Covered by `vite-plus/toolchain`, `vite-plus/versions`. | None.                                                                                    | Low.                                                     | No change |

### Phase 1 selection

Phase 1 has five functions in three entries.

| Item              | In or out | Reason                                                                                                                                                                                          |
| ----------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `findWorkspace()` | In        | Other calls depend on the same detection. The Rust code and the NAPI function exist. An integration can adopt it alone.                                                                         |
| `getTaskGraph()`  | In        | It needs no Vite Task change. It gives editors and monorepo tools typed data at once.                                                                                                           |
| `planRun()`       | In        | It answers "what would run, and is it cached" without side effects. The format exists.                                                                                                          |
| `run()`           | In        | Without it the API cannot run a task, and callers stay on child processes. It has the highest cost, so it ships last in the phase.                                                              |
| `check()`         | In        | `vp check` is the Vite+-owned composite command. It has no structured output in any form today.                                                                                                 |
| `staged()`        | Out       | Git hooks call `vp staged`. No user group above needs it from a script. lint-staged already has a programmatic API.                                                                             |
| `cleanCache()`    | Out       | It is small, but it adds an entry to maintain before any user asked for it.                                                                                                                     |
| Package commands  | Out       | The package-management docs state that `--json` output keeps each package manager's native schema. A typed result needs a shared schema first. `findWorkspace()` covers the detection part now. |
| Create, migrate   | Out       | Both are scripts that prompt and exit the process. An API needs a rewrite into functions first.                                                                                                 |
| `vp env`          | Out       | It belongs to the global CLI. The boundary rule keeps it out of the `vite-plus` package.                                                                                                        |

## Phase 1 API

### Shared conventions

```ts
/** Options that every phase 1 function accepts. */
export interface CommonOptions {
  /**
   * The directory to work from. Relative paths resolve from `process.cwd()`.
   * @default process.cwd()
   */
  cwd?: string;
}

/** Options for the functions that start child processes: `run()` and `check()`. */
export interface ExecOptions extends CommonOptions {
  /**
   * Environment for child processes and for cache fingerprints.
   * @default a copy of `process.env` at call time
   */
  env?: Record<string, string | undefined>;
  /** Cancels the call. Running child processes stop. */
  signal?: AbortSignal;
}

/** The error type for every rejected promise, except an abort. */
export class VitePlusError extends Error {
  /** A stable identifier, such as `ERR_VP_TASK_NOT_FOUND`. */
  code: string;
}
```

Error behavior is the same for all functions:

| Situation                                                    | Behavior                                                                                |
| ------------------------------------------------------------ | --------------------------------------------------------------------------------------- |
| A task exits with a non-zero code, or a check finds issues   | Resolve with `ok: false`                                                                |
| No `package.json` at or above `cwd`                          | Reject with `ERR_VP_NO_WORKSPACE`. `findWorkspace()` resolves with `undefined` instead. |
| A `vite.config.*` file fails to load or has an invalid block | Reject with `ERR_VP_CONFIG`                                                             |
| The task specifier matches no task                           | Reject with `ERR_VP_TASK_NOT_FOUND`                                                     |
| An option value or combination is invalid                    | Reject with `ERR_VP_INVALID_OPTION`                                                     |
| A pinned package manager fails its integrity check           | Reject with `ERR_VP_PACKAGE_MANAGER`                                                    |
| `signal` aborts                                              | Reject with `signal.reason`                                                             |

The error codes are provisional. `error.message` holds the text that the CLI prints after `error:`.

### `vite-plus/workspace`

```ts
import { findWorkspace } from 'vite-plus/workspace';

export function findWorkspace(options?: CommonOptions): Promise<Workspace | undefined>;

export interface Workspace {
  /** Absolute path of the workspace root, or of the nearest package when there is no workspace. */
  root: string;
  /** True when the root has a `pnpm-workspace.yaml` or a `package.json` workspace. */
  isMonorepo: boolean;
  /** Absent when no declaration or file identifies a package manager. */
  packageManager?: {
    name: 'pnpm' | 'npm' | 'yarn' | 'bun';
    /** The declared version or range. Absent when detection used a lockfile or a config file. */
    version?: string;
    /** What identified the package manager. */
    source: 'packageManager' | 'devEngines' | 'lockfile-or-config';
  };
}
```

**Result and errors.** The function resolves with `undefined` when no `package.json` exists at or above `cwd`. It is the one function that does not reject in that case, because "is this a project" is the question it answers. It never downloads a package manager and never writes a file.

**Mapping.**

| Aspect         | Today                                                                                                                                                                            |
| -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CLI            | No command. `vp install` and `vp run` perform this detection. [package-manager-selection.md](../docs/guide/package-management/package-manager-selection.md) documents the order. |
| Implementation | NAPI `detectWorkspace(cwd)` in `packages/cli/binding/src/package_manager.rs`. `packages/cli/src/utils/workspace.ts` already calls it.                                            |
| Work needed    | Return the `PackageManagerSource` value that Rust already computes. Omit the placeholder version `"default"` that lockfile detection returns today.                              |

**Example.** A framework CLI prints the install command for the project.

```ts
import { findWorkspace } from 'vite-plus/workspace';

const workspace = await findWorkspace({ cwd: projectDir });
if (!workspace) {
  throw new Error(`No package.json found from ${projectDir}`);
}
const pm = workspace.packageManager?.name ?? 'pnpm';
console.log(`Run \`${pm} install\` in ${workspace.root}`);
```

The fallback to `pnpm` matches the documented default of `vp install`.

The package list and the dependency edges between packages come from `getTaskGraph()`. See open question 9.

### `vite-plus/run`

```ts
import { getTaskGraph, planRun, run } from 'vite-plus/run';

export function getTaskGraph(options?: CommonOptions): Promise<TaskGraph>;
export function planRun(task: string, options?: PlanOptions): Promise<RunPlan>;
export function run(task: string, options?: RunOptions): Promise<RunResult>;
```

`task` is a task specifier: `build` or `@my/app#build`. It is required. `vp run` without a task opens an interactive selector, and the API never prompts.

#### Options

```ts
/** Task selection. Each option maps to one `vp run` flag. */
export interface SelectOptions {
  /** Arguments for the task process. CLI: the tokens after the task name. */
  args?: string[];
  /** Select all packages. CLI: `-r`, `--recursive`. */
  recursive?: boolean;
  /** Select the current package and its dependencies. CLI: `-t`, `--transitive`. */
  transitive?: boolean;
  /** Select the workspace root package. CLI: `-w`, `--workspace-root`. */
  workspaceRoot?: boolean;
  /** Match packages by name, directory, or glob. CLI: `-F`, `--filter`. */
  filter?: string | string[];
  /** Reject when a filter matches no package. CLI: `--fail-if-no-match`. */
  failIfNoMatch?: boolean;
  /** Skip `dependsOn` tasks. CLI: `--ignore-depends-on`. */
  ignoreDependsOn?: boolean;
  /** `true` is `--cache`. `false` is `--no-cache`. Omitted: the config decides. */
  cache?: boolean;
  /** Run without dependency order. CLI: `--parallel`. */
  parallel?: boolean;
}

export interface PlanOptions extends CommonOptions, SelectOptions {
  /** Environment used for cache fingerprints. Same default as `ExecOptions.env`. */
  env?: Record<string, string | undefined>;
  /** Detail level of cache information. CLI: `-v` is 1, `-vv` is 2. */
  verbose?: 0 | 1 | 2;
}

export interface RunOptions extends ExecOptions, SelectOptions {
  /** CLI: `--concurrency-limit`. The CLI default is 4. */
  concurrencyLimit?: number;
  /**
   * Where task output goes.
   * - `'pipe'`: deliver output through `onEvent`. Nothing reaches the terminal.
   * - `'inherit'`: tasks use the terminal of the calling process, as with `vp run`.
   * - `'ignore'`: discard output.
   * @default 'pipe'
   */
  stdio?: 'pipe' | 'inherit' | 'ignore';
  /** Receives progress events in order. */
  onEvent?: (event: RunEvent) => void;
}
```

The API omits five CLI flags. `--log` selects a text layout. `--last-details` shows the saved summary of the previous run. `--json` and `--dry-run` become `getTaskGraph()` and `planRun()`. `--remote-cache` is hidden in the CLI while remote caching is unreleased.

#### `getTaskGraph()`

The function returns the document that `vp run --json` prints. Vite Task defines it in `crates/vt/schemas/task-graph.json`. As in the CLI, the call writes that schema file to the system temp directory so that `$schema` points to a real file.

```ts
export interface TaskGraph {
  /** `file://` URL of the JSON Schema for this document. */
  $schema: string;
  /** Format version. It changes only for a breaking change. */
  version: 1;
  /** Absolute path. Other paths are relative to it and use `/`. */
  workspaceRoot: string;
  /** Every package, keyed by path. The root is `"."`. */
  packages: Record<string, GraphPackage>;
  /** Every task, keyed by task ID: `<package path>#<task name>`, or the task name for the root. */
  tasks: Record<string, GraphTask>;
}

export interface GraphPackage {
  name?: string;
  /** Workspace packages in `dependencies`, by path. */
  dependencies?: string[];
  devDependencies?: string[];
  peerDependencies?: string[];
}

export interface GraphTask {
  /** The package path. It is a key of `packages`. */
  package: string;
  name: string;
  cwd?: string;
  /** One command, or commands that run in order. */
  command: string | string[];
  /** Task IDs from the task's own `dependsOn`, resolved. */
  dependsOn?: string[];
}
```

**Errors.** The call loads every package's config. It rejects with `ERR_VP_CONFIG` when a config fails to load or a `dependsOn` entry names a task that does not exist.

**Example.** An editor extension builds a task tree.

```ts
import { getTaskGraph } from 'vite-plus/run';

const graph = await getTaskGraph({ cwd: workspaceFolder });
const tree = new Map<string, string[]>();
for (const [id, task] of Object.entries(graph.tasks)) {
  const label = graph.packages[task.package]?.name ?? task.package;
  tree.set(label, [...(tree.get(label) ?? []), id]);
}
```

#### `planRun()`

The function returns the document that `vp run --dry-run --json <task>` prints. Vite Task defines it in `crates/vt/schemas/run-plan.json`. It runs nothing and writes nothing to the cache.

```ts
export interface RunPlan {
  $schema: string;
  version: 1;
  workspaceRoot: string;
  /** The tasks this run would run, keyed by task ID. */
  plan: Record<string, PlanTask>;
}

export interface PlanTask {
  package: string;
  name: string;
  /** IDs of tasks in the same plan that must finish first. */
  dependsOn?: string[];
  /** Each `&&` part, each command array entry, and each `pre` or `post` script is one step. */
  steps: PlanStep[];
}

export type PlanStep = CommandStep | NestedStep;

export interface CommandStep {
  program: string;
  args?: string[];
  cwd?: string;
  /** The `pre` or `post` script that the step comes from. */
  hook?: string;
  /** Absent for a built-in such as `echo`. */
  cache?: PlanCache;
}

/** A nested `vp run` with a plan of its own. */
export interface NestedStep {
  expandedFrom: string;
  plan: Record<string, PlanTask>;
}

export type PlanCache =
  | { status: 'hit'; baseline: PlanBaseline }
  | {
      status: 'changed' | 'unknown';
      baseline?: PlanBaseline;
      changes?: unknown[];
      changesPartial?: true;
    }
  | { status: 'noBaseline' | 'disabled' }
  | { status: 'error'; message: string };

export interface PlanBaseline {
  /** How long the cached run took. On a hit, the time that a replay saves. */
  durationMs: number;
  /** Present with `verbose: 2`. */
  fingerprint?: unknown;
  outputs?: unknown;
}
```

The `unknown` types stand for `Change`, `Fingerprint`, and `Outputs` in the schema. The implementation types them in full. This RFC does not repeat the schema.

**Example.** A CI step skips the test job when every step is a cache hit.

```ts
import { planRun, type PlanStep, type RunPlan } from 'vite-plus/run';

function allHits(plan: RunPlan['plan']): boolean {
  return Object.values(plan).every((task) => task.steps.every(isHit));
}

function isHit(step: PlanStep): boolean {
  if ('plan' in step) {
    return allHits(step.plan);
  }
  return step.cache === undefined || step.cache.status === 'hit';
}

const { plan } = await planRun('test', { recursive: true });
console.log(`skip=${allHits(plan)}`);
```

#### `run()`

The function runs tasks as `vp run <task>` does and resolves with a result object.

```ts
export interface RunResult {
  /** True when every step succeeded or was replayed from the cache. */
  ok: boolean;
  /** The exit code that `vp run` returns for the same run. */
  exitCode: number;
  workspaceRoot: string;
  /** The tasks that the run reached, keyed by task ID. */
  tasks: Record<string, TaskResult>;
}

export interface TaskResult {
  package: string;
  name: string;
  steps: StepResult[];
}

export interface StepResult {
  /** The command text, such as `vitest run`. */
  command: string;
  /** Directory relative to the workspace root. */
  cwd: string;
  outcome: 'success' | 'failed' | 'error' | 'cancelled';
  /** Present when a process ran and exited. */
  exitCode?: number;
  /** Absent for a built-in such as `echo`. */
  cache?: { status: 'hit'; savedMs: number } | { status: 'miss' } | { status: 'disabled' };
  /** Present when `outcome` is `'error'`: a spawn failure or a cache failure. */
  error?: { message: string; causes: string[] };
}

export type RunEvent =
  | { type: 'step:start'; task: string; step: number; cache: 'hit' | 'miss' | 'disabled' }
  | {
      type: 'step:output';
      task: string;
      step: number;
      stream: 'stdout' | 'stderr';
      data: Uint8Array;
    }
  | { type: 'step:finish'; task: string; step: number; result: StepResult };
```

The result shape is a proposal. Vite Task has the data in its internal `LastRunSummary` type, which it saves for `vp run --last-details`. That type is not public and has no format version. The final field names follow a new versioned `run-result` document in Vite Task. See open question 3.

**Behavior.**

- The run stops at the first failure and cancels running tasks, as the CLI does. `result.tasks` holds the tasks that the run reached.
- A cache hit replays the saved output. With `stdio: 'pipe'`, the replayed output arrives as `step:output` events.
- With `stdio: 'inherit'`, a task with caching enabled still has its output captured and forwarded. Vite Task gives a task the terminal directly only when the task has no caching.
- On abort, the run stops as it does for Ctrl-C, and the promise rejects with `signal.reason`. Events that arrived before the abort stay valid.

**Example.** A release script builds the publishable packages and reports failures.

```ts
// scripts/release.ts. Run it with `vpx scripts/release.ts`.
import { run } from 'vite-plus/run';

const result = await run('build', { filter: './packages', stdio: 'inherit' });

if (!result.ok) {
  for (const [id, task] of Object.entries(result.tasks)) {
    const failed = task.steps.find((step) => step.outcome !== 'success');
    if (failed) {
      console.error(`${id} failed: ${failed.command} (exit ${failed.exitCode ?? 'n/a'})`);
    }
  }
  process.exitCode = result.exitCode;
}
```

**Example.** An agent tool runs one task with a time limit and collects its output.

```ts
import { run } from 'vite-plus/run';

const chunks: Uint8Array[] = [];
const result = await run('@my/app#test', {
  args: ['--run'],
  signal: AbortSignal.timeout(120_000),
  onEvent(event) {
    if (event.type === 'step:output') {
      chunks.push(event.data);
    }
  },
});
const log = Buffer.concat(chunks).toString('utf8');
```

#### Mapping

| Function         | CLI                              | Implementation today                                                                | Work needed                                                                                                                                 |
| ---------------- | -------------------------------- | ----------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------- |
| `getTaskGraph()` | `vp run --json`                  | `Session::task_graph_json()`, public in Vite Task. The CLI prints its return value. | A NAPI function that creates a session and serializes the document. No Vite Task change.                                                    |
| `planRun()`      | `vp run --dry-run --json <task>` | A private reporter in Vite Task writes the document to stdout from `Session::main`. | A public Vite Task method that returns the `RunPlan` value.                                                                                 |
| `run()`          | `vp run <task>`                  | `Session::main` prints output and errors. It returns only an exit status.           | In Vite Task: a public reporter or event sink, the `run-result` document, errors as values, and a cancel hook. The pieces exist internally. |

### `vite-plus/check`

```ts
import { check } from 'vite-plus/check';

export function check(options?: CheckOptions): Promise<CheckResult>;

/** Each option maps to one `vp check` flag. */
export interface CheckOptions extends ExecOptions {
  /** Files to check. Omitted: the whole project at `cwd`. */
  paths?: string[];
  /** Format files and apply lint fixes. This changes files. CLI: `--fix`. */
  fix?: boolean;
  /** Also apply dangerous lint fixes and suggestions. CLI: `--fix-dangerously`. */
  fixDangerously?: boolean;
  /** `false` skips the format step. CLI: `--no-fmt`. */
  fmt?: boolean;
  /** `false` skips lint rules. Type check still runs when the config enables it. CLI: `--no-lint`. */
  lint?: boolean;
  /** Report errors only. CLI: `--quiet`. */
  quiet?: boolean;
  /** Treat lint warnings as errors. CLI: `--deny-warnings`. */
  denyWarnings?: boolean;
  /** Run the remaining steps after a step fails. CLI: `--continue-on-error`. */
  continueOnError?: boolean;
  /** CLI: `--no-error-on-unmatched-pattern`. */
  noErrorOnUnmatchedPattern?: boolean;
}
```

`fmt: true` and `lint: true` mean "no flag". They do not override `check.fmt: false` or `check.lint: false` in `vite.config.ts`. The CLI has the same rule.

```ts
export interface CheckResult {
  /** True when no step failed. */
  ok: boolean;
  /** The exit code that `vp check` returns for the same options. */
  exitCode: number;
  fmt: FmtStepResult;
  lint: LintStepResult;
}

/**
 * - `passed`: the step ran and found nothing.
 * - `failed`: the step ran and found issues.
 * - `error`: the tool did not start or did not finish.
 * - `skipped`: an option or the `check` config block turned the step off.
 * - `not-run`: an earlier step failed and `continueOnError` is off.
 */
export type CheckStepStatus = 'passed' | 'failed' | 'error' | 'skipped' | 'not-run';

export interface FmtStepResult {
  status: CheckStepStatus;
  /** The number of files that the formatter checked. */
  files?: number;
  /** Files with formatting issues. Empty after a successful fix. */
  unformatted: string[];
  durationMs?: number;
  /** The tool's output when `status` is `'error'`. */
  output?: string;
}

export interface LintStepResult {
  status: CheckStepStatus;
  /** True when lint rules ran. */
  rules: boolean;
  /** True when type check ran. It needs `lint.options.typeAware` and `lint.options.typeCheck`. */
  typeCheck: boolean;
  files?: number;
  errors: number;
  warnings: number;
  diagnostics: LintDiagnostic[];
  durationMs?: number;
  output?: string;
}

/** One diagnostic, as Oxlint's `--format=json` reporter prints it. */
export interface LintDiagnostic {
  message: string;
  /** The rule, such as `eslint(no-debugger)`. */
  code: string;
  /** Oxlint's severity name in lower case, such as `warning`. */
  severity: string;
  url?: string;
  help?: string;
  filename: string;
  labels: Array<{
    label?: string;
    span: { offset: number; length: number; line: number; column: number };
  }>;
}
```

**Behavior.**

- The step order and the stop rules match [docs/guide/check.md](../docs/guide/check.md). Format runs first. If it fails, lint has the status `not-run` unless `continueOnError` is on.
- With `fix`, the CLI formats, applies lint fixes, and formats again. The `fmt` result describes the final state.
- `diagnostics` holds lint diagnostics. Type check runs through the same Oxlint process, so its diagnostics belong in the same list. This RFC did not verify their shape in the JSON reporter.
- When no step is enabled, the promise rejects with `ERR_VP_INVALID_OPTION`. The CLI prints "No checks enabled" for that case.

**Lint diagnostics.** `vp check` runs oxlint with `--format=default` today and parses the summary lines. The API runs oxlint with `--format=json` and passes the diagnostics through. The `LintDiagnostic` shape above matches the output of oxlint 1.94.0 for one test run with two warnings. This RFC found no upstream statement about the stability of that format. See open question 5.

**Mapping.**

| Aspect         | Today                                                                                                                                                              |
| -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| CLI            | `vp check`, with the flags in `SynthesizableSubcommand::Check` in `packages/cli/binding/src/cli/types.rs`                                                          |
| Implementation | `execute_check` in `packages/cli/binding/src/check/mod.rs`. Parsers in `packages/cli/binding/src/check/analysis.rs`.                                               |
| Work needed    | Split `execute_check` into a function that returns a report and a function that prints it. Add the JSON reporter path for lint. Take `cwd` and `env` as arguments. |

`check()` still starts oxfmt and oxlint as child processes. Oxlint has no JavaScript lint function to call in process. The caller no longer starts a process or parses text.

**Example.** A CI script turns diagnostics into GitHub annotations.

```ts
import { check } from 'vite-plus/check';

const result = await check({ continueOnError: true });

for (const file of result.fmt.unformatted) {
  console.log(`::error file=${file}::File is not formatted. Run vp check --fix.`);
}
for (const diagnostic of result.lint.diagnostics) {
  const span = diagnostic.labels[0]?.span;
  const level = diagnostic.severity === 'error' ? 'error' : 'warning';
  console.log(
    `::${level} file=${diagnostic.filename},line=${span?.line ?? 1},col=${span?.column ?? 1}::${diagnostic.code} ${diagnostic.message}`,
  );
}
process.exitCode = result.exitCode;
```

**Example.** An agent loop fixes what tools can fix, then reads what remains.

```ts
import { check } from 'vite-plus/check';

const result = await check({ fix: true, paths: changedFiles });
const todo = result.lint.diagnostics.map((d) => ({
  file: d.filename,
  line: d.labels[0]?.span.line,
  rule: d.code,
  message: d.message,
  help: d.help,
}));
```

## Implementation

### Where the code lives

| Part                | Location                                                                                                                                             |
| ------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------- |
| Public entries      | `packages/cli/src/workspace.ts`, `packages/cli/src/run.ts`, `packages/cli/src/check.ts`, next to `fmt.ts`, `lint.ts`, and `pack.ts`                  |
| Shared JS code      | A new `packages/cli/src/api/` directory: option validation, `VitePlusError`, and the resolver set that `bin.ts` builds inline today                  |
| NAPI functions      | A new module in `packages/cli/binding/src/`, next to `package_manager.rs` and `migration.rs`                                                         |
| Check report        | `packages/cli/binding/src/check/`                                                                                                                    |
| Task runner changes | The `voidzero-dev/vite-task` repository. Vite+ pins the `vt` crates by git revision in `Cargo.toml`. A change lands there first, then the pin moves. |

The API stays inside the `vite-plus` package. It needs no change to `crates/vp_global_cli`.

### NAPI binding

The existing `run(options)` function shows the pattern. Vite Task futures are not `Send`. The function starts a thread with a current-thread Tokio runtime and a `LocalSet`, and JavaScript callbacks cross threads as `ThreadsafeFunction` values. The new functions use the same pattern. `onEvent` is one more `ThreadsafeFunction`.

Rules for the new NAPI functions:

- **Explicit environment.** Use `Session::init_with(envs, cwd, config)`, which takes the environment and the directory as arguments. Apply the package-manager `PATH` entry and the lifecycle variables to that map. Do not call `prepend_tools_to_path_env` or `stamp_package_manager_lifecycle_env`, which change the process environment.
- **Same argument rules as the CLI.** The JS layer turns typed options into the argument list that the CLI accepts. Rust parses it with the same clap types (`vt::Command`, `SynthesizableSubcommand`). The API and the CLI then reject the same combinations. The clap error becomes `ERR_VP_INVALID_OPTION`.
- **Documents cross as JSON.** The task graph and run plan types implement `Serialize`. The NAPI function returns a JSON string, and the JS layer parses it. No second type mapping in Rust is needed.
- **Same resolvers.** Tasks contain `vp lint`, `vp build`, and similar commands. `VitePlusCommandHandler` turns them into tool invocations through the JS resolver callbacks. The API passes the same callbacks as `bin.ts`.
- **Module load.** The binding's `module_init` clears `O_NONBLOCK` on non-TTY stdio and installs a panic hook. An API import triggers both. Phase 1 must decide whether that is acceptable in a host process. See open question 4.

### Sharing code with the CLI

| Command         | Change                                                                                                                               |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------ |
| `vp check`      | Calls the new report function, then prints the report with the existing `print_*` helpers. Output stays byte-identical.              |
| `vp run`        | No change in phase 1. It keeps `Session::main`. The API uses the new public Vite Task entry points, which `Session::main` also uses. |
| `vp run --json` | No change. The API calls the same `Session::task_graph_json()`.                                                                      |

### Bundling and exports

Per [packages/cli/BUNDLING.md](../packages/cli/BUNDLING.md), every export outside `./test*` is a manual entry in `packages/cli/package.json`.

1. Add `workspace`, `run`, and `check` to the ESM `entry` map in `packages/cli/tsdown.config.ts`. tsdown emits `dist/<name>.js` and `dist/<name>.d.ts`.
2. Add the exports, in the form that `./fmt` uses:

   ```json
   "./run": {
     "types": "./dist/run.d.ts",
     "import": "./dist/run.js"
   }
   ```

3. Keep the binding external. The config already lists `../binding/index.js` under `deps.neverBundle`.
4. Update the export tables in `packages/cli/BUNDLING.md`.

`packages/cli/src/run-config.ts` exists and holds generated config types. The new `run.ts` entry is a different file. Rename one of them if the pair confuses contributors.

### Types

`packages/cli/src/run-config.ts` is generated from Rust, and the test `run_config_types_in_sync` keeps it current. The document types need the same guarantee. Vite Task generates `task-graph.json` and `run-plan.json` from its Rust types and checks them in a test. Phase 1 writes the TypeScript types by hand and adds a test that validates real documents against them. See open question 8.

### Tests

| Layer         | Coverage                                                                                                                                                                                                                                   |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Rust unit     | The check report for each status. Option-to-argument mapping. Environment handling without `set_var`.                                                                                                                                      |
| Vitest unit   | `packages/cli/src/__tests__/`: each function against fixture workspaces. `exports-map.spec.ts` gains the new entries and their types.                                                                                                      |
| Parity        | For one fixture, `getTaskGraph()` equals the parsed output of `vp run --json`, and `planRun()` equals `vp run --dry-run --json`. `check()` returns the exit code of `vp check`.                                                            |
| Silence       | A child process calls each function with `stdio: 'pipe'`. The test asserts that the child's stdout and stderr are empty and that `process.cwd()` and `process.env` are unchanged.                                                          |
| CLI snapshots | Existing `vp check` and `vp run` snapshots must not change. Snapshot steps are argv arrays, and existing fixtures run `["node", "<file>.mjs"]`, so a fixture can call the API from a script. Module resolution in a fixture needs a check. |

### Delivery order

1. `findWorkspace()` and `getTaskGraph()`. No Vite Task change.
2. `planRun()`. One Vite Task method.
3. `check()`. A refactor inside the binding.
4. `run()`. The Vite Task reporter, result document, and cancel hook.

Each step ships with its guide page and carries the experimental label.

## Later Phases

| Phase | API                                                | Notes                                                                                      |
| ----- | -------------------------------------------------- | ------------------------------------------------------------------------------------------ |
| 2     | `staged()` in `vite-plus/staged`                   | Wraps the code in `packages/cli/src/staged/bin.ts`. Returns a boolean today.               |
| 2     | `cleanCache()` in `vite-plus/run`                  | Maps to `vp cache clean`.                                                                  |
| 2     | `install()`, `add()`, `remove()` in `vite-plus/pm` | Start with an exit status only. A typed result needs a schema that Vite+ owns.             |
| 2     | Watch or long-running `run()` handles              | A handle with `close()` for dev servers.                                                   |
| 3     | `create()` and `migrate()`                         | Needs non-interactive functions first. `vp migrate --no-interactive` shows the option set. |

## Non-Goals

- An API for `vp env`, `vp upgrade`, or `vp implode`. The global CLI owns them.
- Any change to the global `vp` CLI.
- New wrappers, new names, or new option shapes for the Vite, Vitest, tsdown, Oxlint, or Oxfmt APIs.
- A lint function in `vite-plus/lint`. Oxlint does not export one. `check()` is the Vite+-owned way to lint from a script.
- One diagnostic model across lint, format, type check, build, and test.
- A shared schema for package-manager output.
- CommonJS builds of the new entries.
- A stable guarantee in phase 1.
- An API for `vp exec`, `vp dlx`, or `vpx`. Node.js `child_process` covers them.

## Alternatives Considered

### JSON output from the CLI only

Add `--json` to more commands and let callers start `vp`. `vp run --json` and `vp run --dry-run --json` already work this way.

Rejected as the only interface. Each caller must find the binary, start a process, parse the output, and write its own types. A process gives no task events and no cancellation that the runner understands. The JSON flags stay, and the API returns the same documents, so both interfaces share one format.

### One generic `vp(args)` function

Export the existing NAPI `run(options)` as `vp(['run', 'build'])`. The cost is close to zero.

Rejected. It returns only an exit code. It prints from Rust to the process stdout. It changes `process.env`. Its options are strings. It solves the location problem and none of the others.

### Export from the root entry

Add `run`, `check`, and `findWorkspace` to `vite-plus`.

Rejected. The root entry has `export * from 'vite'`. A future Vite export with the same name is a conflict that Vite+ cannot control. A root import also loads Vite, which a task panel or a CI script does not need.

### A separate package

Publish `@voidzero-dev/vite-plus-api`.

Rejected. The implementation is the NAPI binding that ships in `vite-plus`. A second package adds a version to keep in step and gains nothing.

### A Vite+ wrapper for lint and format

Add `lint(paths)` and `fmt(paths)` functions that start oxlint and oxfmt and return results.

Rejected. The plan item keeps the Oxlint and Oxfmt APIs unchanged. A wrapper is a second API for the same tool, and it must change when Oxlint ships its own. `check()` covers the composite workflow that Vite+ owns.

### Classes or a long-lived session object

Expose `createSession()` with methods, so several calls reuse one loaded task graph.

Deferred. Functions are enough for the phase 1 workflows. A session object fixes a lifetime and a cache policy before any user needs them. The functions can become methods of a session later without a breaking change.

## Open Questions

1. **Entry name for the task runner.** `vite-plus/run` follows the command and the `run` config block. `vite-plus/task` follows the product name Vite Task and avoids the `run.ts` and `run-config.ts` pair. Which one?
2. **Default `stdio` for `run()`.** `'pipe'` follows principle 3 and Node.js `child_process`. `'inherit'` is what a script author expects from a replacement for `vp run`. Which default?
3. **Run result format.** Does Vite Task add a versioned `run-result` document next to `task-graph` and `run-plan`? Does `vp run` also get a flag that prints it? The proposal assumes yes to the first question.
4. **In-process execution.** `run()` and `check()` work inside the caller's process through the binding. Is that acceptable for long-lived hosts such as an editor extension host? The alternative is a worker thread or a child process that the API manages. A related task: move `ensure_blocking_stdio()` out of `module_init`, or run it only for `stdio: 'inherit'`.
5. **Lint diagnostics source.** Is Oxlint's `--format=json` output stable enough to expose as `LintDiagnostic`? The alternatives are a Vite+-owned normalized type, or `diagnostics` as raw text until Oxlint ships a JavaScript API.
6. **`vite-plus/fmt` completeness.** Does `vite-plus/fmt` re-export `jsTextToDoc`? Oxfmt describes it as a helper for the Prettier `textToDoc()` plugin flow. Does the entry mirror upstream fully, or stay a selected subset?
7. **Import rewrites for `oxfmt` and config-surface `oxlint`.** Are the `vp migrate` rule and the `prefer-vite-plus-imports` change part of this plan item or a separate task?
8. **Type generation.** Hand-written TypeScript types with a validation test, or types generated from the Vite Task JSON Schemas at build time?
9. **Package list without config loading.** `getTaskGraph()` loads every package's config to list packages. Does `findWorkspace()` need a `packages` field that reads only the workspace file and each `package.json`?
10. **Concurrent calls.** Can two `run()` calls work on one workspace at the same time in one process? The task cache is a SQLite database. Phase 1 can queue calls per workspace root or document the limit.
11. **Graduation.** What moves an API from experimental to stable? A proposal: two minor releases without a breaking change, plus a format version on every returned document.
12. **Error codes.** Is the `ERR_VP_*` list in this RFC the right granularity? Vite Task errors arrive as message chains today, so a code needs a mapping in the binding.
