# RFC: Run TypeScript Scripts with `vpx`

- Status: Draft (for review)
- Related: [`vpx` command](./vpx-command.md), [JavaScript runtime management](./js-runtime.md), [CLI bundling](../packages/cli/BUNDLING.md), [Core binding resolution](./core-binding-resolution.md)
- Upstream: [oxc-project/oxc-node](https://github.com/oxc-project/oxc-node) (`@oxc-node/core` 0.1.3; source vendored and compiled into the Vite+ native binding, as Rolldown is)
- Prior art: [tsx](https://github.com/privatenumber/tsx) 4.23.15

## Summary

Let `vpx` run a script file directly:

```bash
vpx ./scripts/seed.ts --dry-run
```

When the command names a script file, `vpx` runs it with the project's managed Node.js and the [oxc-node](https://github.com/oxc-project/oxc-node) loader preloaded, instead of resolving a package binary. The loader's native half is compiled into the Vite+ native binding the way Rolldown is; its JavaScript half ships in `vite-plus/dist`. TypeScript syntax that Node.js cannot strip (enums, namespaces, parameter properties, decorators, JSX) works without a `tsx`, `ts-node`, or `esno` dependency, and so do tsconfig `paths`, `.js` → `.ts` imports, and ESM/CJS interop. Package-binary execution (`vpx eslint .`) is unchanged.

```
vpx ./seed.ts args  ──►  node --import file:///…/vite-plus/dist/script-register.js ./seed.ts args
                         (project-resolved Node.js; exec'd in place on Unix, so one process)
```

The feature is available from the global shim and from a new project-local `vpx` bin, so `package.json` scripts can use it in CI without the global CLI.

## Motivation

Script runners are among the most common devDependencies. Repositories use `tsx` to run `scripts/*.ts` for seeding, code generation, and releases. Vite+ already manages the Node.js version and ships the rest of the toolchain, but running a TypeScript file still means choosing between:

```bash
pnpm add -D tsx && pnpm tsx scripts/seed.ts   # extra dependency (esbuild), two Node.js processes
vp node scripts/seed.ts                       # Node.js type stripping only
vpx tsx scripts/seed.ts                       # downloads tsx when it is not installed
```

Node.js type stripping (unflagged since 22.18 / 23.6) only erases syntax. It rejects enums, namespaces with runtime code, parameter properties, decorators, and JSX. It ignores tsconfig, so `paths` aliases do not resolve. It requires explicit `.ts` extensions and rejects TypeScript inside `node_modules`. Scripts written for tsx usually depend on at least one of these.

VoidZero maintains oxc-node, an Oxc-based Node.js loader. This repository already uses it for its own build scripts: `oxnode -C dev ./build.ts` in `packages/cli` and `packages/core`, and `import '@oxc-node/core/register'` in `packages/tools/src/bin.js`. Rolldown runs its TypeScript scripts the same way. Exposing it through `vpx` gives users a tsx equivalent that needs no extra dependency and uses the project-pinned Node.js.

### Proposed Solution

```bash
vpx ./scripts/seed.ts --dry-run                       # Run a TypeScript file
vpx scripts/build.mts                                 # A bare relative path works when the file exists
vpx --watch ./server.ts                               # Node.js options before the script are forwarded
vpx --tsconfig tsconfig.scripts.json ./tools/gen.ts   # Pick a tsconfig explicitly
```

```ts
#!/usr/bin/env vpx
// scripts/release.ts: chmod +x, then run as ./scripts/release.ts
// (the file keeps its .ts extension; see "Shebang Scripts" under Edge Cases)
```

## Background

### tsx

tsx 4.23.15 (2026-09-20) is the de facto standard. The behavior relevant to this design:

- **CLI**: `tsx [tsx and Node.js options] <file> [args]`. Its own options are `--tsconfig`, `--no-cache`, `-v`, and `-h`. It transforms `-e`/`-p` code with esbuild and forwards every other option to Node.js unchanged. `tsx watch` is a separate subcommand (chokidar with `--include`, `--exclude`, and `--clear-screen`). Running `tsx` without arguments opens a REPL that transforms TypeScript input.
- **Process model**: the tsx CLI is a Node.js process. It spawns `process.execPath --require preflight.cjs --import loader.mjs …` as a child, relays signals through IPC over a Unix socket (a named pipe on Windows), and exits with the child's code or 128 plus the signal number.
- **Hooks**: CJS uses `Module._extensions` and `Module._resolveFilename`. ESM uses `module.registerHooks()` on Node.js 22.22.3+, 24.11.1+, and 26, and `module.register()` on 18.19+ and 20.6+.
- **Transform**: esbuild compiles each file in isolation, with `isolatedModules` semantics. Output is cached on disk under `os.tmpdir()/tsx-<uid>` and kept for about eight days.
- **Resolution**: `.js` → `.ts`, extensionless imports, directory `index` files, and tsconfig `paths`. tsx reads one tsconfig per process, found from the cwd. TypeScript in `node_modules` is compiled.
- **Unsupported**: type checking, `emitDecoratorMetadata`, and top-level `await` in CJS-scoped files.
- **API**: `node --import tsx`, `tsx/esm/api` (`register`, `tsImport`), and `tsx/cjs/api`.

### oxc-node

- **Package**: `@oxc-node/core` 0.1.3 is a NAPI-RS addon published as 16 platform packages (about 4.2 MB unpacked for `darwin-arm64`). Its only JS dependency is `pirates`. Usage: `node --import @oxc-node/core/register file.ts`.
- **Hooks** (`register.mjs`): CJS goes through pirates (`Module._extensions` for `.js .jsx .ts .tsx .mjs .mts .cjs .cts .es6 .es`). ESM uses `module.registerHooks()` on Node.js ≥ 26.2 and the off-thread `module.register()` on older versions. The register module also turns on source map support.
- **Resolution**: oxc_resolver with `extension_alias` (`.js` → `.ts`/`.tsx`, `.mjs` → `.mts`, `.cjs` → `.cts`) and tsconfig `paths`. Each file uses the nearest tsconfig that includes it, as tsc does. `OXC_TSCONFIG_PATH` or `TS_NODE_PROJECT` overrides this.
- **Transform**: oxc_transformer. Honored options:
  - `experimentalDecorators` and `emitDecoratorMetadata`
  - `useDefineForClassFields`
  - `jsxImportSource`, `jsxFactory`, and `jsxFragmentFactory`
  - `rewriteRelativeImportExtensions`
- **Limits**: no lowering for `target` and no transform cache. Files in `node_modules` are skipped unless `OXC_TRANSFORM_ALL` is set.
- **`oxnode` CLI**: a thin wrapper that spawns `node` from `PATH`, so it ignores the managed runtime. It does not forward signals and reports exit code 0 when the child is killed by a signal. Code passed to `-e` and typed into the REPL is not transformed. Vite+ will not use it.
- **Maturity**: the [oxc.rs docs](https://oxc.rs/docs/guide/usage/oxc-node.html) mark oxc-node as experimental. CI covers Node.js 22, 24, and 26 on Linux, macOS, and Windows.

### Node.js Type Stripping

Stripping is on by default since 22.18 / 23.6 and stable since 24.12 / 25.2. `--experimental-transform-types` was removed in Node.js 26. Only erasable syntax is supported, and Node.js reads no tsconfig settings. `vp node file.ts` already gives users this behavior.

## Goals

1. `vpx <script>` runs `.ts .mts .cts .tsx .js .mjs .cjs .jsx` files with full TypeScript syntax, tsconfig `paths`, `.js` → `.ts` imports, and both module systems.
2. Scripts run on the Node.js that `vp node` would select: project version resolution, `vp env off` system-first mode, and inherited runtimes inside `vp run`.
3. On Unix, the script runs in a single process. Exit codes, signals, stdio, and `process.ppid` behave as they do with `node`.
4. Node.js options such as `--watch`, `--inspect`, `--env-file=`, `--test`, and `--import` are forwarded.
5. `#!/usr/bin/env vpx` works as a shebang for files that keep a script extension (`./release.ts`).
6. The loader ships inside `vite-plus`: native code in the existing platform binding, JavaScript in `dist`. No new npm package, and users add no dependency.
7. `vpx <script>` works without the global CLI, from a project-local `vite-plus` install.
8. Package-binary behavior of `vpx` stays unchanged.

## Non-Goals (v1)

- Type checking. Use `vp check` or `tsc --noEmit`.
- A TypeScript-aware REPL and TypeScript in `-e`/`-p`. `vp node -e` still applies Node.js type stripping.
- A tsx-style `watch` subcommand with `--include` and `--exclude`. Use Node.js `--watch` and `--watch-path`.
- A programmatic API (`tsImport`) or a public `node --import vite-plus/…` entry point. See [Future Enhancements](#future-enhancements).
- An on-disk transform cache.
- `target` downleveling.
- Bun and Deno runtimes.
- Overlap with `vp test` (Vitest) or `vp run` (tasks).

## Command Syntax

```bash
vpx [VPX_OPTIONS] [NODE_OPTIONS] <script> [args...]
```

- `VPX_OPTIONS`: the existing `vpx` options, plus `--tsconfig <path>` and `-v/--version`. `-p/--package` and `-c/--shell-mode` select package mode, so combining them with a script is an error. `-s/--silent` is accepted; script mode prints no Vite+ output in either case.
- `NODE_OPTIONS`: tokens starting with `-` between the `vpx` options and the script. They are forwarded to Node.js verbatim, before the script.
- `<script>` and `[args...]`: passed to Node.js unchanged, as with `node <script> [args...]`. Tokens after the script, including `--`, belong to the script.

`vpx` options are parsed first, so short options that Node.js also defines (`-p`, `-c`, `-h`, `-v`) keep their `vpx` meaning. Node.js equivalents are available as long options (`--print`, `--check`, `--help`, `--version`). `vpx --version` prints the Vite+ version, as `vp --version` does; today it reaches `pnpm dlx --version`, which nobody can rely on.

`--tsconfig` is recognized anywhere before the script, not only before the first Node.js option, because `vpx --watch --tsconfig x.json ./a.ts` is a natural way to write it. The other `vpx` options keep the current rule: they must come first, and the first unrecognized option starts the Node.js options.

**Node.js invocations without a script.** When the first positional token starts with `-` and no script token follows, `vpx` forwards the arguments to Node.js with the loader and lets Node.js interpret them. This covers `vpx --eval '…'`, `vpx --print '…'`, `vpx --check ./a.ts`, `vpx -` (stdin), and `vpx --inspect` (which opens the Node.js REPL). The loader does not transform eval or REPL input, so these behave like `vp node` plus hooks for the files they import. There is no "expected a script" error.

### Usage Examples

```bash
# Run a TypeScript file
vpx ./scripts/seed.ts --dry-run

# Bare relative path (the file must exist)
vpx scripts/seed.ts

# Node.js options
vpx --watch ./server.ts
vpx --inspect-brk ./debug-me.ts
vpx --env-file=.env ./scripts/migrate.ts
vpx --import ./setup.ts ./main.ts

# Explicit tsconfig
vpx --tsconfig tsconfig.scripts.json ./tools/gen.ts

# Package mode (unchanged)
vpx eslint .
vpx oxlint@1.85.0 --version
```

## Script Detection

Detection runs before any package-spec parsing, such as `@version` detection, because script paths can contain `@`.

```text
after parsing vpx options, let T = positional[0]

if T starts with '-':                      # Node.js invocation; never a package bin today
    script = first token after the Node.js options where is_script(token),
             skipping the value of options that take a separate argument
             (--import, --require, --loader, --experimental-loader, --watch-path,
              --env-file, --inspect-port, --title, --conditions, -C, -r, …)
    none found → forward to Node.js as-is (eval, print, stdin, REPL, --check)
elif is_script(T):
    script = T
else:
    package mode (existing lookup chain, unchanged)

is_script(token):
    explicit = token starts with ./ ../ / (Windows: .\ ..\ X:\ \\)
    ext      = token ends with .ts .mts .cts .tsx .js .mjs .cjs .jsx
    ext && explicit                     → script (error if missing; never downloads)
    ext && cwd/token is a regular file  → script
    ext ∈ TS extensions && missing      → error "script not found" (no package fallback)
    explicit && regular file whose first line is a shebang that invokes vpx → script
    otherwise                           → not a script
```

Detection decides the mode and which token gets the missing-file check. It never rewrites the arguments: Node.js receives every token in order, after `--import <loader>`, and parses its own options. The skip list exists only so that `vpx --import ./setup.ts ./main.ts` checks `./main.ts` rather than `./setup.ts`; the list is a best effort, and a wrong guess only moves the missing-file check to Node.js, which reports `Cannot find module` itself.

The `T starts with '-'` branch is backward compatible. Today the first unknown option becomes the command name (`parse_vpx_args` in `crates/vp_global_cli/src/commands/vpx.rs`), so `vpx --inspect ./a.ts` ends up running `pnpm dlx --inspect ./a.ts`, which never ran a package.

| Invocation                                 | Mode    | Reason                                                              |
| ------------------------------------------ | ------- | ------------------------------------------------------------------- |
| `vpx ./scripts/seed.ts`                    | script  | Explicit path with a script extension                               |
| `vpx scripts/seed.ts`                      | script  | The file exists                                                     |
| `vpx seed.ts` (no such file)               | error   | TypeScript extension; npm package names ending in `.ts` are ignored |
| `vpx ./missing.js`                         | error   | Explicit path; never falls back to a remote download                |
| `vpx highlight.js` (no such file)          | package | Bare name with a JS extension and no file                           |
| `vpx ./@scope/tool.ts`                     | script  | Detection runs before `@version` parsing                            |
| `vpx --env-file .env ./a.ts`               | script  | Leading option; `./a.ts` is the first script token                  |
| `vpx --import ./setup.ts ./main.ts`        | script  | `./setup.ts` is the value of `--import`; `./main.ts` is the script  |
| `vpx --eval 'console.log(1)'`              | node    | No script token; forwarded to Node.js with the loader               |
| `vpx /abs/bin/tool` (`#!/usr/bin/env vpx`) | script  | Shebang guard; otherwise `vpx` would exec the file and loop forever |
| `vpx ./deploy.sh`                          | package | No script extension; existing `PATH` lookup executes it             |
| `vpx eslint .`                             | package | Unchanged                                                           |

## Execution Model

```text
vpx ./scripts/seed.ts --dry-run
 │
 ├─ parse vpx options (--tsconfig, -s, -v; -p/-c select package mode)
 ├─ detect script ── package ──► existing chain: node_modules/.bin → global → PATH → dlx
 │        │ script / node-only
 ├─ resolve loader: <local vite-plus>/dist/script-register.js, else <global vite-plus>/dist/…
 ├─ env: prepend node_modules/.bin (cwd → root); VP_SCRIPT_TSCONFIG if --tsconfig
 └─ core `node` shim dispatch (same runtime selection as `vp node`)
        └─ execve: node --import <loader URL> [NODE_OPTIONS] ./scripts/seed.ts --dry-run
                     └─ script-register.js: Node.js version check → load vite-plus binding → register hooks
```

### Runtime Selection

Script mode hands the rewritten argv to the core `node` shim path in `crates/vp_global_cli/src/shim/dispatch.rs`, which `vp node` / `vp env exec node` also use. Script runs therefore get project version resolution (`.node-version`, `devEngines.runtime`, `engines.node`, …), system-first mode, `VP_BYPASS`, inherited runtimes inside `vp run`, and the `node`/`npm` `PATH` preparation.

### Process Model

On Unix, `exec_tool` replaces the `vpx` process with `node` through `execve`. No parent process remains, so no IPC, signal relay, or exit-code translation is needed. This is the main structural difference from tsx. On Windows, there is no `execve`: the `vpx.exe` trampoline starts `vp.exe`, which spawns `node` and waits, as for every shim today. Exit codes and Ctrl+C there follow the existing `node` shim behavior.

The project-local `vpx` bin (see [Design Decision 8](#8-why-ship-a-project-local-vpx-bin)) is JavaScript, so it is a Node.js parent process that spawns `process.execPath --import <loader> …` with inherited stdio, forwards `SIGINT`/`SIGTERM`, and exits with the child's code or `128 + signal`. The global shim remains the single-process path.

### Loader Entry

`vite-plus` gains an internal entry point, `dist/script-register.js` (built from `packages/cli/src/script-hooks/register.ts`). `vpx` passes it to `--import` as a `file://` URL, which is required for Windows drive paths. The entry point:

1. Checks the Node.js version and exits with a Vite+ error if the version is outside the supported range (see [Open Questions](#open-questions)). Node.js older than 20.6 has no `--import`, so those versions fail earlier with Node.js's own `bad option` message.
2. Loads the Vite+ native binding through `packages/cli/binding/index.js`, the same loader the CLI uses, and registers the hooks: the pirates CommonJS hook, then `module.registerHooks()` on Node.js ≥ 26.2 or `module.register('./script-esm-hooks.js')` below it. The ESM hooks module loads the binding again inside the loader worker thread; the binding loads in about 2 ms, so this is not a concern.
3. Maps runtime helper imports to `@oxc-project/runtime`, which `@voidzero-dev/vite-plus-core` already depends on (see [Native Hooks in the Binding](#3-native-hooks-in-the-binding)).

The contract between the CLI and the loader is `--import <file URL of dist/script-register.js>` plus the `VP_SCRIPT_TSCONFIG` environment variable. Because the global `vp` may be newer or older than the project's `vite-plus`, this contract must stay stable across versions; new behavior goes into the loader entry, not into new argv.

The loader is resolved from the project first and the global install second, following the global CLI's existing JS-entry delegation:

1. The project's `vite-plus` package (`resolve_local_vite_plus_package_dir` in `crates/vp_global_cli/src/js_executor.rs`), if it contains `dist/script-register.js`.
2. The global install (`JsExecutor::get_scripts_dir()`, `<DATA>/<version>/node_modules/vite-plus/dist`).

### Environment

- Every `node_modules/.bin` from the cwd up to the filesystem root is prepended to `PATH`, as for local binaries today (`prepend_node_modules_bin_to_path`). Scripts can then spawn project tools by name.
- `--tsconfig <path>` is resolved against the cwd and must exist. `vpx` passes it to the child as `VP_SCRIPT_TSCONFIG`. The flag changes more than the file: without it, the hooks select the nearest tsconfig that includes each file; with it, one config applies to the whole process, including files that config does not include, for both resolution and transform. The documentation says so. The vendored hooks read only `VP_SCRIPT_TSCONFIG`; upstream's `OXC_TSCONFIG_PATH` and `TS_NODE_PROJECT` are not consulted, so values left in the environment by other tools cannot change the result.
- The loader is passed in argv, not `NODE_OPTIONS`. `process.execArgv` carries it, so `child_process.fork('./worker.ts')` and `new Worker('./worker.ts')` inherit it. Unrelated Node.js processes the script spawns, such as `vp build` or `npm`, do not load it.
- Other loaders are unsupported. A `NODE_OPTIONS=--import tsx` in the environment, or a user `--import tsx`, registers a second set of hooks; Node.js runs the most recently registered hooks first, and the result depends on which loader claims the file. `vpx` does not detect or strip these.

## Comparison with tsx

| Capability                     | tsx 4.23                          | `vpx` + oxc-node 0.1.3                                            | Node.js type stripping |
| ------------------------------ | --------------------------------- | ----------------------------------------------------------------- | ---------------------- |
| `.ts .mts .cts`                | ✓                                 | ✓                                                                 | ✓                      |
| `.tsx` / `.jsx`                | ✓                                 | ✓ (automatic runtime; classic needs a fix)                        | ✗                      |
| Enums, namespaces, param props | ✓                                 | ✓                                                                 | ✗                      |
| Legacy decorators              | ✓                                 | ✓                                                                 | ✗                      |
| `emitDecoratorMetadata`        | ✗                                 | ✓                                                                 | ✗                      |
| TC39 decorators                | ✓ (esbuild lowers)                | ✗ (upstream)                                                      | ✗                      |
| tsconfig `paths`               | ✓                                 | ✓                                                                 | ✗                      |
| tsconfig selection             | One per process, from the cwd     | Nearest per file (tsc rules)                                      | n/a                    |
| `.js` → `.ts`, extensionless   | ✓                                 | ✓                                                                 | ✗                      |
| `require()` of `.ts`, interop  | ✓ (compiles ESM to CJS if needed) | ✓ (relies on Node.js `require(esm)`)                              | Partial                |
| TypeScript in `node_modules`   | ✓                                 | ✗ (v1; see Vendored Changes)                                      | ✗                      |
| Type checking                  | ✗                                 | ✗ (`vp check`)                                                    | ✗                      |
| Watch                          | `tsx watch` (chokidar)            | Node.js `--watch`                                                 | `--watch`              |
| TypeScript REPL / `-e`         | ✓                                 | ✗ (v1)                                                            | Erasable syntax only   |
| Transform cache                | On disk, about 8 days             | None                                                              | n/a                    |
| Processes per run              | 2 (Node.js parent + child)        | 1 on Unix (Rust execs Node.js); 2 via the local bin or on Windows | 1                      |
| Node.js version                | Whatever `node` runs tsx          | Project-managed                                                   | Project-managed        |
| Shebang                        | `#!/usr/bin/env tsx`              | `#!/usr/bin/env vpx` (file keeps `.ts`)                           | `#!/usr/bin/env node`  |
| Programmatic API               | `tsImport`, `register`            | ✗ (future)                                                        | n/a                    |
| Extra install                  | `tsx` + esbuild                   | None (compiled into the `vite-plus` binding)                      | None                   |

## Upstream Status and Vendored Changes

These items in oxc-node were found by reading the 0.1.3 source and testing it. Because the source is vendored, Vite+ can fix each one in its copy and send the fix upstream; the vendored copy should carry as few patches as possible, so the default is upstream first and sync after.

### Fixed by Vendoring

1. **Runtime helper resolution.** oxc-node always lowers class fields, static blocks, and `using`, and it lowers legacy decorators when enabled. Lowered code imports helpers such as `@oxc-node/core/helpers/defineProperty`. These resolve relative to the user's file, so a project that does not depend on `@oxc-node/core` fails with `ERR_MODULE_NOT_FOUND`. Because class fields are lowered unconditionally, any script that declares `class X { count = 0 }` hits this; it is the common case, not an edge case. This reproduces on 0.1.3, and the upstream tests work around it by symlinking the package into fixtures.

   In the vendored copy, `HelperLoaderOptions::module_name` becomes `@oxc-project/runtime`, which `@voidzero-dev/vite-plus-core` already depends on at the oxc version pinned in the catalog. The resolve hook rewrites `@oxc-project/runtime/helpers/*` specifiers to absolute `file://` URLs under core's package directory, computed once at register time, so helper resolution never depends on the user's `node_modules`. The same fix (resolve helpers relative to the loader, not the importer) is proposed upstream.

### Verify, Then Fix

2. **Helpers in CommonJS output.** The CJS (pirates) path emits helper imports as ESM `import` statements. If that holds for `.cts` and CJS-scoped `.ts` files, any class field in a CJS file is a syntax error, and CJS cannot be a v1 feature until it is fixed. Fix in the vendored copy and upstream.

### Upstream First, Then Sync

3. **Do not lower what Node.js already supports.** Class fields, private members, static blocks, and `using` are lowered regardless of the running Node.js, which changes `Function.prototype.toString()`, stack frames, and performance, and is the source of item 1. Lower only syntax the target Node.js lacks.
4. Node.js 25.9–26.1 emit DEP0205 for `module.register()`, and Vite+ supports `>=26.0.0`. Use `registerHooks()` wherever it is safe, or avoid the warning.
5. Map tsconfig `jsx` values (`react`, `react-jsx`, `react-jsxdev`, `preserve`). Only the strings `automatic` and `classic` are matched today, so the classic runtime cannot be selected with valid TypeScript values.
6. Honor `verbatimModuleSyntax` (`only_remove_type_imports` is hard-coded to `false`).
7. Lower TC39 standard decorators. Node.js has no native decorators, so these currently throw a `SyntaxError`.
8. Transform `.ts/.mts/.cts/.tsx` inside `node_modules` without `OXC_TRANSFORM_ALL`, which transforms every file there.
9. Support Yarn PnP. Enable the `yarn_pnp` feature of oxc_resolver, as Vite+ already does.
10. Stop freezing the resolver's `condition_names` from the first resolve call.

### Nice to Have

11. Transform TypeScript in `--eval`, stdin, and the REPL.
12. An opt-in persistent transform cache.
13. TypeScript syntax in extensionless entry files, for shebang scripts without `.ts`.
14. Split the crate into a NAPI-free core and a thin NAPI layer, so the vendored copy shrinks to the glue and the patches in [Native Hooks in the Binding](#3-native-hooks-in-the-binding) become unnecessary.

## Implementation Architecture

### 1. Detection and Dispatch

**File**: `crates/vp_global_cli/src/commands/vpx.rs`

- `parse_vpx_args`: add `-v/--version`, and `--tsconfig <path>` / `--tsconfig=<path>`. `--tsconfig` is also extracted from among the Node.js options.
- New `detect_script(&positional, cwd) -> Result<ScriptMode, ScriptError>`, called in `execute_vpx` before `extract_command_name` and `has_version_spec`. `ScriptMode` is `Package`, `Script(token)`, or `NodeOnly`.
- New `execute_script(invocation, flags, cwd)`. It builds `["--import", loader_url, ...positional]` and the child `ToolPathEnv`, then calls the core `node` shim dispatch.

The detection rules are pure functions over `(tokens, cwd)` and are unit tested in the same module.

### 1b. Project-Local `vpx` Bin

**Files**: `packages/cli/bin/vpx`, `packages/cli/src/bin.ts`, `packages/cli/package.json`

- Add `"vpx": "./bin/vpx"` next to `vp` and `vpr`. The shim imports `dist/bin.js`, which detects `argv0 === 'vpx'` the way it detects `vpr` today.
- Script mode in JS: the same detection rules, then `spawn(process.execPath, ['--import', loaderUrl, ...rest], { stdio: 'inherit' })` with signal forwarding and exit-code propagation. `process.execPath` is already the project-resolved Node.js when the bin was started through the `node` shim or a package manager script.
- Package mode delegates to the global `vp` through `VP_CLI_BIN` when it is set, and otherwise prints a new hint that package execution with `vpx` requires the global CLI (`vp exec` in `packages/cli/binding/src/exec/workspace.rs` already points users at `vpx` for remote commands, so the two messages must agree).
- The detection rules live in one place per language; the Rust and TS unit tests share the table in this RFC.

### 2. Loader Resolution

**File**: `crates/vp_global_cli/src/js_executor.rs`

Add `resolve_script_register(cwd) -> Result<AbsolutePathBuf, Error>`. It tries the project's `vite-plus` package and then the global scripts dir, reusing `resolve_local_vite_plus_package_dir` and `get_scripts_dir`.

### 3. Native Hooks in the Binding

**Files**: `packages/tools/.upstream-versions.json`, `packages/tools/src/sync-remote-deps.ts`, `Cargo.toml`, `packages/cli/binding/Cargo.toml`, `packages/cli/binding/src/lib.rs`, `packages/cli/build.ts`

oxc-node is vendored the way Rolldown is. `sync-remote` already clones `rolldown` and `vite` into gitignored directories at the commit pinned in `.upstream-versions.json`; it gains an `oxc-node` entry that clones into `oxc-node/`. The workspace adds `oxc_node = { path = "./oxc-node" }`, and the binding does `pub extern crate oxc_node;` next to `rolldown_binding`, so every `#[napi]` export of oxc-node (`transform`, `transformAsync`, `createResolve`, `load`, `OxcTransformer`) lands in `vite-plus.<platform>.node`.

What the binding already contains makes this cheap. Through `rolldown_binding`, the release binding links the same crates oxc-node uses (`oxc` with transformer, codegen, and semantic; `oxc_resolver` 11.24.3; `oxc_sourcemap` 9) and already exports `transform`, `transformSync`, `resolveTsconfig`, and `ResolverFactory`. oxc-node's `src/lib.rs` (about 1,550 lines) and `src/windows_file_url.rs` (about 1,170 lines) are glue over those crates. The binding grows by well under 1 MB.

The vendored copy needs these patches, applied by `sync-remote` after checkout so that the upstream tree stays diffable:

| Patch                                                                                     | Reason                                                                                                                                                              |
| ----------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Remove `#[global_allocator]` (`src/lib.rs`)                                               | `rolldown_binding` already declares one; two in one `cdylib` fail to link                                                                                           |
| Remove the `module_init` that installs a `tracing_subscriber` from `OXC_LOG`              | Vite+ owns tracing; a second global subscriber panics on install                                                                                                    |
| Point `oxc`, `oxc_resolver`, `oxc_sourcemap`, `napi`, `napi-derive` at `workspace = true` | One `Cargo.lock`: oxc-node tracks oxc head (0.153.0 today) while Rolldown and Vite+ are on 0.152.0; the vendored commit must compile against the workspace versions |
| `HelperLoaderOptions::module_name` → `@oxc-project/runtime`                               | See Fixed by Vendoring item 1                                                                                                                                       |
| Read `VP_SCRIPT_TSCONFIG` instead of `OXC_TSCONFIG_PATH` / `TS_NODE_PROJECT`              | See [Environment](#environment)                                                                                                                                     |
| Enable `oxc_resolver`'s `yarn_pnp` feature                                                | Already on in the workspace; oxc-node leaves it off                                                                                                                 |

The `rolldown` Cargo feature is release-only, so `pnpm build` produces a binding without oxc. oxc-node must be in dev builds too, or `vpx ./x.ts` and the local-flavor snapshot tests do not work on a development checkout. The binding gets an `oxc-node` feature that is on by default and compiles oxc-node in both build types; the cost is the oxc crates in dev builds (see [Open Questions](#open-questions)).

### 3b. Loader JavaScript

**Files**: `packages/cli/src/script-hooks/register.ts`, `packages/cli/src/script-hooks/esm.ts`, `packages/cli/package.json`, `packages/cli/build.ts`

- `register.ts` and `esm.ts` are adapted from oxc-node's `packages/core/register.mjs` (160 lines) and `esm.mjs` (26 lines). They import the native functions from `../binding/index.js` instead of `@oxc-node/core`, add the Node.js version check, and add the helper specifier rewrite.
- `pirates` (the CommonJS `Module._extensions` hook, pure JavaScript, no dependencies) is added to `vite-plus` `dependencies` or inlined by the bundler.
- Build outputs: `dist/script-register.js` (the `--import` entry) and `dist/script-esm-hooks.js` (passed to `module.register()` on Node.js < 26.2).
- `@oxc-project/runtime` stays a dependency of `@voidzero-dev/vite-plus-core`; the loader resolves its directory once from core's package root. The catalog pins `@oxc-project/runtime` and `@oxc-project/types` to one exact version, which must equal the `oxc` crate version compiled into the binding. `sync-remote` already resolves oxc-related version conflicts toward the higher version; this adds the Rust side to that check.

### 4. Toolchain Metadata and Docs

- `packages/cli/toolchain.config.json`: register `oxc-node` with `delivery: ["compiled"]`, as Rolldown's compiled half is, so `vp --version` and `vp toolchain` report it with the vendored commit's version.
- `packages/cli/BUNDLING.md`: add an "oxc-node Native Binding Integration" section parallel to the Rolldown one (feature flag, vendored patches, export list).
- `docs/guide/vpx.md`: add a "Running scripts" section. `docs/guide/env.md`: contrast `vp node` (plain Node.js) with `vpx <script>`.
- Update the `vpx --help` text (`VPX_HELP`).

## CLI Help Output

```bash
$ vpx --help
Execute a command from a local or remote npm package, or run a script file

Usage: vpx [OPTIONS] <pkg[@version]> [args...]
       vpx [OPTIONS] [NODE_OPTIONS] <script> [args...]

Arguments:
  <pkg[@version]>  Package binary to execute
  <script>         Script file to run (.ts, .mts, .cts, .tsx, .js, .mjs, .cjs, .jsx)
  [args...]        Arguments to pass to the command or script

Options:
  -p, --package <NAME>  Package(s) to install if not found locally
  -c, --shell-mode      Execute the command within a shell environment
  -s, --silent          Suppress all output except the command's output
      --tsconfig <PATH> tsconfig.json to use when running a script
  -v, --version         Print the Vite+ version
  -h, --help            Print help

Examples:
  vpx eslint .                         # Run local eslint (or download)
  vpx create-vue my-app                # Download and run create-vue
  vpx ./scripts/seed.ts --dry-run      # Run a TypeScript script
  vpx --watch ./server.ts              # Pass Node.js options before the script
```

## Error Handling

```bash
$ vpx ./scripts/missing.ts
error: Script not found: ./scripts/missing.ts

$ vpx -p cowsay ./a.ts
error: --package cannot be used when running a script

$ vpx --tsconfig tsconfig.missing.json ./a.ts
error: tsconfig not found: tsconfig.missing.json

$ vpx ./a.ts          # .node-version pins 20.19.0
error: Running scripts with vpx requires Node.js ^22.18.0 || ^24.11.0 || >=26.0.0 (current: v20.19.0)
```

Errors raised in the script, including transform errors, come from Node.js and oxc-node unchanged. Source maps are on, so stack traces point at the TypeScript source.

## Design Decisions

### 1. Why `vpx` Instead of a New Command

**Decision**: Extend `vpx`. Leave `vp node` as plain Node.js.

**Rationale**:

- `vpx` is the "run this" entry point. `vpx ./a.ts` reads like `npx tsx ./a.ts` without the extra package.
- `#!/usr/bin/env vpx` is short, and the `vpx` shim is already on `PATH` after `vp env setup`.
- `vp node` is documented as `vp env exec node`. It stays the escape hatch for exact Node.js semantics, including type stripping without a loader.

### 2. Why oxc-node

**Decision**: Use oxc-node instead of Node.js type stripping, tsx, or pre-bundling.

**Rationale**: it covers the TypeScript syntax and resolution features that scripts rely on. It shares Oxc with the rest of the toolchain (Oxlint, Oxfmt, Rolldown), is maintained in the same organization, and is already used by this repository and by Rolldown. See [Alternatives Considered](#alternatives-considered).

### 3. Why Exec Node.js Directly

**Decision**: Replace the `vpx` process with `node` and do not keep a supervising parent.

**Rationale**:

- tsx needs a Node.js parent because its CLI is JavaScript. The Vite+ shim is native, so it can `execve` once version resolution is done.
- Exit codes, signals, TTY handling, and `process.ppid` are exactly those of `node`. Vite+ needs no IPC protocol or signal-relay heuristics.
- It saves one Node.js startup per run.

### 4. Why a Vite+-Owned Loader Entry

**Decision**: Pass `--import <vite-plus>/dist/script-register.js` instead of `--import @oxc-node/core/register`.

**Rationale**:

- It provides one place for the Node.js version check, the helper specifier rewrite, and binding loading.
- The oxc-node version follows the `vite-plus` version, which the project pins.
- The hooks implementation can change without changing the CLI contract.
- It can later become a public `node --import vite-plus/register` entry (see [Future Enhancements](#future-enhancements)).

### 5. Why the Loader Goes in argv Instead of `NODE_OPTIONS`

**Decision**: Pass the loader as an argv flag.

**Rationale**: `process.execArgv` carries it into `fork()` and worker threads, which may load TypeScript. Unrelated Node.js processes the script spawns, such as package managers and `vp` itself, do not load it. tsx behaves the same way.

### 6. Why Explicit Paths Never Download

**Decision**: An explicit path, or a bare name with a TypeScript extension, is never treated as a package spec.

**Rationale**: a typo such as `vpx ./scirpt.ts` should report a missing file. Today it can fall through to `vp dlx`, which runs remote code under an unexpected name.

### 7. Why Resolve the Loader From the Project First

**Decision**: Use the project's `vite-plus` when it contains `dist/script-register.js`, and fall back to the global install.

**Rationale**: the global CLI already delegates JS entry points this way. Teams and CI then run the hooks and binding of the `vite-plus` version pinned in the lockfile. The cost is the version-skew contract described under [Loader Entry](#loader-entry).

### 8. Why Ship a Project-Local `vpx` Bin

**Decision**: `vite-plus` ships a `vpx` bin alongside `vp` and `vpr`.

**Rationale**:

- Without it, `"seed": "vpx ./scripts/seed.ts"` in `package.json` works on a developer machine and fails in CI that only runs `npm i -D vite-plus`. tsx is a devDependency and has no such gap, so a tsx replacement must not introduce one.
- `vp migrate` can only rewrite `tsx ./x.ts` to `vpx ./x.ts` if the result works wherever `tsx` worked.
- `vpr` already set the precedent for a local shorthand bin that detects itself by `argv0`.
- The local bin covers script mode and delegates package mode. It does not reimplement the lookup chain.

### 9. Why Compile oxc-node Into the Binding

**Decision**: Vendor the oxc-node source and compile it into `vite-plus.<platform>.node`, as Rolldown is, instead of depending on the `@oxc-node/core` npm package.

**Rationale**:

- The crates are already there. Through `rolldown_binding`, the release binding links `oxc` (transformer, codegen, semantic), `oxc_resolver`, and `oxc_sourcemap` and exports `transform`, `resolveTsconfig`, and `ResolverFactory`. oxc-node adds about 2,700 lines of glue and no new crates.
- No new packages. `@oxc-node/core` would add a 17-target platform matrix (about 4 MB per platform) next to Vite+'s 8-target one, with its own install, version-check, and WASI-fallback paths.
- Load cost is negligible. Measured on this machine (Node.js 22.18, macOS arm64): the 42 MB release binding loads in 2.3–3.2 ms warm and 17.9 ms cold; the 4.2 MB `@oxc-node/core` binding loads in 0.7–1.1 ms warm. Transform speed is identical, since it is the same oxc.
- The blocking helper bug stops being an upstream dependency. With the hooks in Vite+, helper imports map to `@oxc-project/runtime`, which core already ships.
- One oxc version for the whole toolchain. Rolldown, `vp lint`'s type-aware parsing, the static config extractor, and the script loader cannot drift apart.
- The vendoring mechanism (`sync-remote`, `.upstream-versions.json`, workspace path deps, a Cargo feature, publish-time platform injection) exists and is exercised on every release.

**Costs**: oxc-node tracks oxc head and bumps often (18 oxc updates in its changelog), so the vendored commit must be chosen to compile against the workspace's oxc version, and the vendored copy carries the small patch set listed in [Native Hooks in the Binding](#3-native-hooks-in-the-binding). Dev builds compile the oxc crates.

## Alternatives Considered

### A. Node.js Type Stripping Only

`vpx a.ts` would behave like `vp node a.ts`.

- **Pros**: no dependency and no loader cost.
- **Cons**: no enums, decorators, JSX, `paths`, or `.js` → `.ts` resolution, and it requires Node.js ≥ 22.18.

It also adds nothing over `vp node`, which remains available.

### B. Bundle tsx

- **Pros**: mature and feature-complete.
- **Cons**: it adds esbuild as a second transformer to an Oxc-based toolchain. It also brings tsx's two-process model and signal relay, and runs on whatever Node.js starts its CLI.

### C. Pre-Bundle With Rolldown

This would mirror Vite's `configLoader: 'bundle'`.

- **Pros**: handles everything at build time, including TypeScript in `node_modules`.
- **Cons**:
  - It changes `import.meta.url` and `__dirname` semantics.
  - Dynamic imports with runtime-computed paths break.
  - It writes temporary files and is slow for one-off scripts.

Vite itself added the `runner` and `native` loaders because of these issues.

### D. Depend on `@oxc-node/core` From npm

Add `@oxc-node/core` to `vite-plus` `dependencies`, as `oxlint` and `oxfmt` are, and `--import` its `register` entry.

- **Pros**: no vendoring, no Rust changes, upstream fixes arrive with a version bump.
- **Cons**:
  - A second native addon with its own 17-target platform matrix, install path, and version checks, next to the binding that already links the same oxc crates. About 4 MB per platform.
  - The helper-resolution bug (Fixed by Vendoring item 1) blocks shipping until upstream fixes it, and the workaround from JavaScript is uncertain because of hook ordering.
  - oxc-node's oxc version floats independently of the rest of the toolchain.

Rejected in favor of Design Decision 9. This was the RFC's first draft.

### E. Use the `oxnode` CLI

It spawns `node` from `PATH` (ignoring the managed runtime), does not forward signals, and reports exit code 0 when the child is killed by a signal. It also costs an extra Node.js process.

### F. Reimplement the Hooks in TypeScript on the Existing Binding Exports

Write `resolve`/`load` hooks in TypeScript against `transformSync`, `ResolverFactory`, and `resolveTsconfig`, which the binding already exports through Rolldown.

- **Pros**: no vendored Rust at all, and oxc lockstep for free.
- **Cons**: reimplements format detection, the ESM-in-CommonJS re-parse, JSON module synthesis, and Windows file-URL handling that oxc-node has tests for; several NAPI calls per file instead of one.

Kept as the fallback if vendoring proves harder than expected.

## Edge Cases

### Shebang Scripts

`#!/usr/bin/env vpx` makes the kernel run `vpx /abs/path/tool.ts args`. The absolute path is explicit, so script mode applies. This is the supported form: the file keeps its `.ts` extension and is made executable.

An extensionless executable with that shebang (`bin/tool`) does not work for TypeScript in v1. Node.js treats extensionless files as JavaScript, and oxc-node's hooks only claim known extensions, so the file runs as plain JavaScript (Nice to Have item 13). The shebang check in `is_script` exists only to stop a loop: without it, `vpx /abs/bin/tool` would go through the `PATH` lookup, exec the file, and re-enter `vpx` forever. Options in a shebang need `#!/usr/bin/env -S vpx --tsconfig …`.

### Scripts Outside the Current Directory

The Node.js runtime and `node_modules/.bin` are resolved from the cwd, matching `vp node`. Imports resolve from the script's location (Node.js semantics), and each file uses its nearest tsconfig.

### `--require` Preloads

Node.js runs `--require` preloads before `--import` preloads. A user's `--require ./setup.ts` therefore runs before the loader is registered, while `--import ./setup.ts` works. The prototype either documents this or also preloads the loader through `--require` (see [Open Questions](#open-questions)).

### `TS_NODE_PROJECT` and `OXC_TSCONFIG_PATH`

Upstream oxc-node reads both, with `TS_NODE_PROJECT` taking precedence, so a value left over from ts-node changes tsconfig selection. The vendored hooks read neither; only `VP_SCRIPT_TSCONFIG`, set by `vpx --tsconfig`, overrides per-file selection.

### Windows

There are no shebangs. `vpx scripts\a.ts` works through the file-exists rule, and the loader is passed as a `file:///C:/…` URL. Exit codes and Ctrl+C follow the existing `node` shim spawn-and-wait path.

### Inside `vp run` and `package.json` Scripts

With the global CLI installed, `node_modules/.bin/vpx` (the project-local bin) shadows the global shim on `PATH` inside `vp run` and package manager scripts. Both resolve the same loader and the same Node.js, so the difference is the extra parent process of the local bin. Without the global CLI, only the local bin exists and package mode prints the existing "requires the global CLI" hint.

## Testing Strategy

### Unit Tests

The detection table and option parsing in `crates/vp_global_cli/src/commands/vpx.rs` get unit tests, and the version check in `script-register.ts` gets a `pnpm test:unit` test.

### Snapshot Tests

A new fixture, `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/command_vpx_script/`, declares its cases for both flavors (`vp = ["local", "global"]`), so the global shim and the project-local bin are checked against the same snapshots. Unlike `command_vpx_pnpm10`, these cases need a built `packages/cli/dist` and a dev-built binding with the `oxc-node` feature in both flavors: the runner installs the checkout package into each case home, and script mode loads `dist/script-register.js` and the native binding from it. The runner's stale-`dist` guard covers only the local flavor, so `just snapshot-test-global` cannot run these cases without a prior `pnpm build`; the fixture README notes this. Cases:

| Case                               | Covers                                                                                                     |
| ---------------------------------- | ---------------------------------------------------------------------------------------------------------- |
| `vpx_script_ts_syntax`             | Enum, namespace, parameter property, class field initializer (helper regression), decorators with metadata |
| `vpx_script_resolution`            | `.js` → `.ts`, extensionless import, tsconfig `paths`, `.mts`/`.cts`, JSON, `require()` of `.ts`           |
| `vpx_script_jsx`                   | `.tsx` with `jsxImportSource` pointing at a fixture-local runtime                                          |
| `vpx_script_args_and_exit_code`    | `process.argv`, arguments after `--`, `process.exitCode = 7` propagation                                   |
| `vpx_script_node_options`          | `--env-file=.env`, `--env-file .env`, `--import ./setup.ts ./main.ts`, `--eval` without a script           |
| `vpx_script_tsconfig`              | `--tsconfig` first and after `--watch`, override of per-file selection, the missing-file error             |
| `vpx_script_errors`                | Missing script, `-p` with a script, unsupported Node.js version                                            |
| `vpx_script_shebang` (non-Windows) | `chmod +x` with `./tool.ts`, and the extensionless recursion guard                                         |
| `vpx_script_watch`                 | PTY: `--watch`, edit an imported `.ts` file, restart milestone                                             |
| `vpx_package_mode_unchanged`       | `vpx --help`, bare package names, `pkg@version`                                                            |

### Prototype Verification Checklist

These need confirmation on Node.js 22.18, 24, and 26 before implementation:

- The vendored crate links into the binding: allocator and `module_init` patches applied, one `oxc` version in `Cargo.lock`, `cargo build` time for dev builds measured.
- The helper specifier rewrite to `@oxc-project/runtime` works for class fields, `using`, and legacy decorators, from ESM and from CommonJS files.
- Class fields in `.cts` and CJS-scoped `.ts` files: do helper imports produce a syntax error (Verify, Then Fix item 2)?
- `VP_SCRIPT_TSCONFIG` drives both the resolver and the transform tsconfig.
- The binding loads inside the `module.register()` worker thread on Node.js < 26.2 through `packages/cli/binding/index.js`.
- `--watch` restarts when an imported `.ts` file changes, on both the sync (`registerHooks`) and async (`register`) hook paths.
- `--test` with `.ts` test files, where each file runs in a subprocess that inherits `execArgv`.
- `require('./x.ts')` from CJS, and `require()` of ESM `.ts` files.
- The pirates CJS hook combined with Node.js's own `.ts` handling on Node.js ≥ 22.18.
- `child_process.fork('./w.ts')` and `new Worker('./w.ts')`.
- Signal forwarding and `128 + signal` exit codes through the project-local bin.
- Windows: loader URL, Ctrl+C, and exit codes.

## Performance

Preliminary startup medians for a hello-world `.ts` on Node.js 22.18 (async hook path, 15 runs on one machine) show the loader in line with Node.js type stripping and ahead of tsx:

| Command                                          | Median  |
| ------------------------------------------------ | ------- |
| `node hello.js`                                  | 17.7 ms |
| `node hello.ts` (type stripping)                 | 46.2 ms |
| `node --import @oxc-node/core/register hello.ts` | 46.6 ms |
| `tsx hello.ts`                                   | 59.1 ms |

The prototype will re-measure on Node.js 22, 24, and 26, including the overhead of the Vite+ shim (version resolution is cached) and a graph of about 1,000 modules. The second measurement decides whether a transform cache (Nice to Have item 12) is needed.

Binding load cost, measured by `require()` of the `.node` file alone on Node.js 22.18, macOS arm64:

| Binding                               | Size   | Warm load  | Cold load |
| ------------------------------------- | ------ | ---------- | --------- |
| `vite-plus.darwin-arm64.node` (1.1.0) | 42 MB  | 2.3–3.2 ms | 17.9 ms   |
| `oxc-node.darwin-arm64.node` (0.1.3)  | 4.2 MB | 0.7–1.1 ms | 736 ms    |

The 2 ms difference is within the noise of the startup numbers above, so compiling the hooks into the Vite+ binding costs nothing a user can notice. The cold numbers are first-touch page cache effects, not representative.

## Security Considerations

1. **No remote fallback for files**: explicit paths, and bare names with TypeScript extensions, never reach `vp dlx`.
2. **Trusted loader**: the hooks are compiled into the `vite-plus` binding and its `dist`; nothing is loaded from the project's `node_modules` except `@oxc-project/runtime` helpers from core's own dependency tree.
3. **Shebang recursion guard**: prevents an exec loop for extensionless `#!/usr/bin/env vpx` files.
4. **No new environment surface**: `VP_SCRIPT_TSCONFIG` is set only when `--tsconfig` is passed, and the hooks ignore `TS_NODE_PROJECT` and `OXC_TSCONFIG_PATH`.

## Backward Compatibility

Package mode is unchanged. Behavior changes only for invocations that run a file:

- `vpx <existing file with a script extension>` now runs the file. Previously an executable file with a shebang was exec'd through the `PATH` lookup, and a non-executable file fell through to `vp dlx`, which failed.
- A bare name that matches both a local file and a package (`vpx highlight.js` with `./highlight.js` present) now runs the file, as `node` and `tsx` would.
- Invocations whose first token after the `vpx` options starts with `-` previously reached `vp dlx` with that token as the package name and never ran a package. They now run Node.js. `vpx --version` previously printed the package manager's version through `pnpm dlx --version`; it now prints the Vite+ version.
- `vite-plus` gains a `vpx` bin. Projects that already have a different `vpx` in `node_modules/.bin` (there is no such package on npm today) would see a bin conflict warning from their package manager.
- No new packages. The platform binding grows by well under 1 MB; `vite-plus` gains `pirates` (pure JavaScript) as a dependency unless it is inlined.

## Rollout

1. **Phase 0, vendor and prototype**: add `oxc-node` to `sync-remote` at a commit that compiles against the workspace's oxc, apply the patch set, link it into the binding, port the two hook files, run the verification checklist, and benchmark. Send the helper-resolution fix upstream.
2. **Phase 1, experimental**: detection, execution, `--tsconfig`, the project-local bin, snapshot tests, and docs marked experimental.
3. **Phase 2, stable**: remove the experimental label and teach `vp migrate` to rewrite `tsx`, `ts-node`, and `esno` usages in `package.json` scripts.
4. **Phase 3**: see Future Enhancements.

## Open Questions

1. **Command surface**: is `vpx <script>` enough, or should `vp exec <script>` also run scripts? Should `vp node` stay plain Node.js? (Proposed: `vpx` only, and `vp node` stays plain.)
2. **Bare names**: should `vpx seed.ts`, without `./`, run an existing file? Should a missing bare `foo.ts` be an error or a package lookup? (Proposed: run the file, and report an error.)
3. **JavaScript files**: should `.js`, `.mjs`, `.cjs`, and `.jsx` also get the loader, for example so that `.js` can import `.ts`? (Proposed: yes.)
4. **Node.js range**: should script mode require the Vite+ engines range (`^22.18.0 || ^24.11.0 || >=26.0.0`), or anything with `module.register()` (≥ 20.6)? Node.js 20 reached end of life in April 2026. (Proposed: the engines range.)
5. **Loader source**: project `vite-plus` first, or always the global install? (Proposed: project first.)
6. **Local bin scope**: Design Decision 8 ships a project-local `vpx` that handles script mode and delegates package mode. Should it instead implement the full lookup chain in JS, so that `vpx eslint .` also works without the global CLI? (Proposed: delegate; keep one implementation of the chain.)
7. **`--require` ordering**: should the loader also be preloaded with `--require`, so that `--require ./setup.ts` works? That requires a CJS-loadable entry.
8. **Extensionless TypeScript in shebang scripts**: wait for upstream support, or document the `.ts` requirement? (Proposed: document; see Edge Cases.)
9. **oxc version lockstep**: oxc-node follows oxc head; Rolldown and Vite+ lag by a release or two. Pin the vendored oxc-node commit to whatever compiles against the workspace's oxc, or let `sync-remote` rewrite its `oxc*` dependencies to `workspace = true` and carry compile fixes when oxc's API moved? (Proposed: rewrite to workspace versions; pick commits that need no compile fixes.)
10. **Maturity**: oxc-node is marked experimental. Should v1 ship behind an "experimental" docs label, or wait for an oxc-node 1.0?
11. **`--version`**: should `vpx --version` print the Vite+ version (proposed), or should it stay a Node.js option so that `vpx --version` prints the Node.js version like tsx's second line?
12. **Dev builds**: compile oxc-node into dev builds by default (slower `cargo build`, but `vpx ./x.ts` works from a checkout), or keep it release-only like Rolldown and have dev builds fall back to the `@oxc-node/core` npm package that the workspace already installs? (Proposed: default on; measure the build-time cost in Phase 0.)

## Future Enhancements

- **Public loader entry**: `node --import vite-plus/register ./a.ts`, for tools that must start `node` themselves (parity with `node --import tsx`).
- **TypeScript eval and REPL**: `vpx -e '<ts>'` and a TypeScript-aware REPL, once oxc-node transforms eval input.
- **Watch ergonomics**: `--include` and `--exclude` globs, screen clearing, and restart-on-keypress beyond Node.js `--watch`.
- **Transform cache**: opt-in, keyed like tsx's (source, options, transformer version).
- **Programmatic API**: a `tsImport()` equivalent exported from `vite-plus`.
- **`vp migrate` support**: replace `tsx`, `ts-node`, `esno`, and `jiti` script invocations and remove those devDependencies.

## References

- tsx source (v4.23.15): https://github.com/privatenumber/tsx/tree/v4.23.15/src
- tsx docs: https://tsx.is
- oxc-node source: https://github.com/oxc-project/oxc-node (`packages/core/register.mjs`, `packages/core/esm.mjs`, `src/lib.rs`, `src/windows_file_url.rs`)
- oxc-node docs: https://oxc.rs/docs/guide/usage/oxc-node.html
- Rolldown vendoring in this repository: `packages/tools/src/sync-remote-deps.ts`, `packages/tools/.upstream-versions.json`, `packages/cli/BUNDLING.md` ("Rolldown Native Binding Integration")
- Node.js TypeScript support: https://nodejs.org/api/typescript.html
- Node.js module customization hooks: https://nodejs.org/api/module.html#customization-hooks
- Rolldown usage of oxc-node: https://github.com/rolldown/rolldown/blob/main/packages/rolldown/tests/cli/cli-e2e.test.ts
