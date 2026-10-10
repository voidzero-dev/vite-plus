# RFC: Run TypeScript Scripts with `vpx`

- Status: Draft (for review); prototype implemented and verified on Node.js 22, 24, and 26 (macOS)
- Related: [`vpx` command](./vpx-command.md), [JavaScript runtime management](./js-runtime.md), [CLI bundling](../packages/cli/BUNDLING.md), [Core binding resolution](./core-binding-resolution.md)
- Upstream: [oxc-project/oxc-node](https://github.com/oxc-project/oxc-node) (v0.1.6; source vendored and compiled into the Vite+ native binding, as Rolldown is)
- Prior art: [tsx](https://github.com/privatenumber/tsx) 4.23.15

## Summary

Let `vpx` run a script file directly:

```bash
vpx ./scripts/seed.ts --dry-run
```

When the command names a script file, `vpx` runs it with the project's managed Node.js and the [oxc-node](https://github.com/oxc-project/oxc-node) loader preloaded, instead of resolving a package binary. The loader's native half is compiled into the Vite+ native binding the way Rolldown is; its JavaScript half ships in `vite-plus/dist`. TypeScript syntax that Node.js cannot strip (enums, namespaces, parameter properties, legacy decorators, JSX) works without a `tsx`, `ts-node`, or `esno` dependency, and so do tsconfig `paths`, `.js` → `.ts` imports, TypeScript dependencies, and ESM/CJS interop. Package-binary execution (`vpx eslint .`) is unchanged.

```
vpx ./seed.ts args  ──►  node --require …/vite-plus/dist/script-preload.cjs
                              --import file:///…/vite-plus/dist/script-register.js ./seed.ts args
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
- **Resolution**: oxc_resolver with `extension_alias` (`.js` → `.ts`/`.tsx`, `.mjs` → `.mts`, `.cjs` → `.cts`) and tsconfig `paths`. Each file uses the nearest tsconfig that includes it, as tsc does. `setTsconfigPath()` (from v0.1.6), `TS_NODE_PROJECT`, or `OXC_TSCONFIG_PATH` overrides this, in that order.
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

`vpx` options are parsed first, so short options that Node.js also defines (`-p`, `-c`, `-h`, `-v`) keep their `vpx` meaning. Node.js equivalents are available as long options (`--print`, `--check`, `--help`), and `vp node --version` prints the Node.js version. `vpx --version` prints the same report as `vp --version`; today it reaches `pnpm dlx --version`, which nobody can rely on.

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
 ├─ parse vpx options (--tsconfig, -s, -v, -h; -p/-c select package mode)
 ├─ detect script ── package ──► existing chain: node_modules/.bin → global → PATH → dlx
 │        │ script / node-only
 ├─ resolve loader: <local vite-plus>/dist, else <global vite-plus>/dist
 ├─ env: prepend node_modules/.bin (cwd → root); set or clear VP_SCRIPT_TSCONFIG
 └─ core `node` shim dispatch (same runtime selection as `vp node`)
        └─ execve: node --require <dist/script-preload.cjs>
                        --import <file URL of dist/script-register.js>
                        [NODE_OPTIONS] ./scripts/seed.ts --dry-run
```

### Runtime Selection

Script mode hands the rewritten argv to the core `node` shim path in `crates/vp_global_cli/src/shim/dispatch.rs`, which `vp node` / `vp env exec node` also use. Script runs therefore get project version resolution (`.node-version`, `devEngines.runtime`, `engines.node`, …), system-first mode, `VP_BYPASS`, inherited runtimes inside `vp run`, and the `node`/`npm` `PATH` preparation.

### Process Model

On Unix, `exec_tool` replaces the `vpx` process with `node` through `execve`. No parent process remains, so no IPC, signal relay, or exit-code translation is needed. This is the main structural difference from tsx. On Windows, there is no `execve`: the `vpx.exe` trampoline starts `vp.exe`, which spawns `node` and waits, as for every shim today. Exit codes and Ctrl+C there follow the existing `node` shim behavior.

The project-local `vpx` bin (see [Design Decision 8](#8-why-ship-a-project-local-vpx-bin)) is JavaScript, so it is a Node.js parent process that spawns `process.execPath` with the same loader flags and inherited stdio. It ignores `SIGINT`, which reaches the child through the process group, forwards `SIGTERM` and `SIGHUP`, and exits the way the child did (re-raising its signal on Unix, so callers see `128 + signal`). The global shim remains the single-process path.

### Loader Entries

The loader has two entries in `vite-plus/dist`, passed together:

1. **`--require <dist/script-preload.cjs>`** (built as CommonJS from `packages/cli/src/script-preload.ts`). It runs before any user `--require` or `--import`, so `--require ./setup.ts` preloads work too. It:
   - checks the Node.js version and exits with a Vite+ error outside the supported range (being CommonJS, it can do that on every Node.js version);
   - loads the Vite+ native binding through `packages/cli/binding/index.cjs`;
   - turns on source maps and installs the pirates CommonJS hook, which also covers TypeScript under `node_modules`;
   - on Node.js 22.22.3+, 24.11.1+, and 26+, registers the in-thread ESM hooks with `module.registerHooks()`.
2. **`--import <file URL of dist/script-register.js>`** (ESM). On older 22.x and 24.x releases it registers the off-thread ESM hooks (`dist/script-esm-hooks.js`) with `module.register()`; Node.js 22 cannot call `module.register()` while a `--require` is loading. On every version, an `--import` also makes Node.js run the entry point through the ESM loader, where the hooks decide each file's module format.

The in-thread hooks start about twice as fast as a `module.register()` hooks thread, and Node.js 25.9+ deprecates `module.register()` (DEP0205). They need two fixes: nodejs/node#59011 (resolve `conditions`, in 22.19 and 24.5) and nodejs/node#59929 (a null source for CommonJS from a sync load hook, in 22.22.3 and 24.11.1). tsx switches at the same releases. Upstream oxc-node waits for 26.2 for nodejs/node#62920, which only affects load hooks that return source for CommonJS, which these hooks never do. Measured with the project-local bin, `vpx` on a two-module `.mts` script went from 78 to 57 ms on Node.js 22.23 and from 69 to 52 ms on 24.11 (medians of 15 runs).

The contract between the CLI and the loader is these two flags plus the `VP_SCRIPT_TSCONFIG` environment variable. Because the global `vp` may be newer or older than the project's `vite-plus`, this contract must stay stable across versions; new behavior goes into the loader entries, not into new argv.

The loader is resolved from the project first and the global install second, following the global CLI's existing JS-entry delegation:

1. The project's `vite-plus` package (`JsExecutor::resolve_local_vite_plus_package_dir`), if its `dist/` contains both entries.
2. The global install (`JsExecutor::get_scripts_dir()`, `<DATA>/<version>/node_modules/vite-plus/dist`).

### Environment

- Every `node_modules/.bin` from the cwd up to the filesystem root is prepended to `PATH`, as for local binaries today (`prepend_node_modules_bin_to_path`). Scripts can then spawn project tools by name.
- `--tsconfig <path>` is resolved against the cwd and must exist. `vpx` passes it to the child as `VP_SCRIPT_TSCONFIG`, and clears an inherited value when the flag is absent. The flag changes more than the file: without it, the hooks select the nearest tsconfig that includes each file; with it, one config applies to the whole process, including files that config does not include, for both resolution and transform. The preload passes it to oxc-node's `setTsconfigPath()`, which takes precedence over upstream's `TS_NODE_PROJECT` and `OXC_TSCONFIG_PATH`. Without the flag, those variables apply as they do for `oxnode`. The [guide](../docs/guide/vpx.md#tsconfig) lists how each compiler option behaves when no tsconfig applies to a file.
- The loader is passed in argv, not `NODE_OPTIONS`. `process.execArgv` carries it, so `child_process.fork('./worker.ts')` and `new Worker('./worker.ts')` inherit it. Unrelated Node.js processes the script spawns, such as `vp build` or `npm`, do not load it.
- Other loaders are unsupported. A `NODE_OPTIONS=--import tsx` in the environment, or a user `--import tsx`, registers a second set of hooks; Node.js runs the most recently registered hooks first, and the result depends on which loader claims the file. `vpx` does not detect or strip these.

## Comparison with tsx

| Capability                     | tsx 4.23                          | `vpx` (vendored oxc-node 0.1.6)                                                                  | Node.js type stripping |
| ------------------------------ | --------------------------------- | ------------------------------------------------------------------------------------------------ | ---------------------- |
| `.ts .mts .cts`                | ✓                                 | ✓                                                                                                | ✓                      |
| `.tsx` / `.jsx`                | ✓                                 | ✓ (automatic and classic runtimes)                                                               | ✗                      |
| Enums, namespaces, param props | ✓                                 | ✓                                                                                                | ✗                      |
| Legacy decorators              | ✓                                 | ✓                                                                                                | ✗                      |
| `emitDecoratorMetadata`        | ✗                                 | ✓                                                                                                | ✗                      |
| TC39 decorators                | ✓ (esbuild lowers)                | ✗ (oxc has no transform; clear error)                                                            | ✗                      |
| tsconfig `paths`               | ✓                                 | ✓ for `import`; not for `require()`                                                              | ✗                      |
| tsconfig selection             | One per process, from the cwd     | Nearest per file (tsc rules)                                                                     | n/a                    |
| `.js` → `.ts`, extensionless   | ✓                                 | ✓                                                                                                | ✗                      |
| `require()` of `.ts`, interop  | ✓ (compiles ESM to CJS if needed) | ✓ (`.cts` with ESM syntax is an error)                                                           | Partial                |
| TypeScript in `node_modules`   | ✓                                 | ✓                                                                                                | ✗                      |
| Native class fields, `using`   | Lowered to `target`               | Native unless `[[Set]]` fields or legacy decorators; `using` lowered on 22.x                     | Native                 |
| Type checking                  | ✗                                 | ✗ (`vp check`)                                                                                   | ✗                      |
| Watch                          | `tsx watch` (chokidar)            | Node.js `--watch`                                                                                | `--watch`              |
| TypeScript REPL / `-e`         | ✓                                 | ✗ (v1)                                                                                           | Erasable syntax only   |
| Transform cache                | On disk, about 8 days             | None; faster than tsx's warm cache except for megabyte-sized files ([Performance](#performance)) | n/a                    |
| Processes per run              | 2 (Node.js parent + child)        | 1 on Unix (Rust execs Node.js); 2 via the local bin or on Windows                                | 1                      |
| Node.js version                | Whatever `node` runs tsx          | Project-managed                                                                                  | Project-managed        |
| Shebang                        | `#!/usr/bin/env tsx`              | `#!/usr/bin/env vpx` (file keeps `.ts`)                                                          | `#!/usr/bin/env node`  |
| Programmatic API               | `tsImport`, `register`            | ✗ (future)                                                                                       | n/a                    |
| Extra install                  | `tsx` + esbuild                   | None (compiled into the `vite-plus` binding)                                                     | None                   |

## Upstream Status and Vendored Changes

These items were found by reading oxc-node (0.1.3, then v0.1.4) and by running the prototype on Node.js 22.18, 24.11, 26.0, and 26.5. The default is upstream first and sync after. Of the 13 issues filed, [oxc-node#794](https://github.com/oxc-project/oxc-node/issues/794) to [oxc-node#798](https://github.com/oxc-project/oxc-node/issues/798) were fixed in v0.1.5, and [oxc-node#804](https://github.com/oxc-project/oxc-node/issues/804) to [oxc-node#806](https://github.com/oxc-project/oxc-node/issues/806) and [oxc-node#808](https://github.com/oxc-project/oxc-node/issues/808) to [oxc-node#811](https://github.com/oxc-project/oxc-node/issues/811) in v0.1.6, which the vendored copy pins. [oxc-node#807](https://github.com/oxc-project/oxc-node/issues/807) is open.

### Fixed Upstream

- **Enum evaluation** ([oxc-node#795](https://github.com/oxc-project/oxc-node/issues/795), v0.1.5). oxc's enum lowering reads member values that only `SemanticBuilder::with_enum_eval(true)` computes; without it, a string enum alias (`Default = Theme.Light`) got a reverse mapping that overwrote the aliased member (oxc#21667).
- **tsconfig `jsx` values** ([oxc-node#796](https://github.com/oxc-project/oxc-node/issues/796), v0.1.5). `react` selects the classic runtime and `react-jsxdev` the automatic runtime in development mode; every other value, including `preserve` and `react-native`, which Node.js cannot run, selects the automatic runtime.
- **Package `type` for `.tsx` and `.jsx`** ([oxc-node#797](https://github.com/oxc-project/oxc-node/issues/797), v0.1.5). oxc_resolver applies it to `.js`/`.ts` only, so a `.tsx` in a `"type": "module"` package ran as CommonJS.
- **`verbatimModuleSyntax`** ([oxc-node#798](https://github.com/oxc-project/oxc-node/issues/798), v0.1.5) keeps imports whose bindings are unused at runtime, with their side effects.
- **Runtime helper resolution** ([oxc-node#794](https://github.com/oxc-project/oxc-node/issues/794), v0.1.5). Upstream tags the helper module name per loader copy and resolves `@oxc-node/core@<tag>/helpers/*` from that copy, through its resolve hook and a `Module._resolveFilename` patch. The binding accepts any helper module name, but no name alone resolves from every user file in both module systems, so Vite+ keeps its own mapping (items 1 and 2 below).
- **Lower only what Node.js lacks** ([oxc-node#808](https://github.com/oxc-project/oxc-node/issues/808), v0.1.6). Class fields, private members, and static blocks are lowered only where native semantics differ: `[[Set]]` fields (`useDefineForClassFields: false`) and legacy decorators. `using` is lowered only below Node.js 24, detected when the addon loads. Native class features need no runtime helpers, so most scripts import none.
- **Standard decorators** ([oxc-node#809](https://github.com/oxc-project/oxc-node/issues/809), v0.1.6) fail with an error that names `experimentalDecorators` instead of Node.js' `SyntaxError` at the `@`. Lowering them needs a transform oxc does not have yet.
- **Export conditions per request** ([oxc-node#810](https://github.com/oxc-project/oxc-node/issues/810), v0.1.6). The shared resolver kept the conditions of its first caller, which are empty when a `require()` transform runs first; each condition set now gets its own resolver, sharing the base resolver's caches.
- **`.cts` files with ES module syntax** ([oxc-node#811](https://github.com/oxc-project/oxc-node/issues/811), v0.1.6) fail with one error on every load path, suggesting a rename to `.mts` or `import x = require()` and `export =`. Node.js rejects such a file too. tsc and tsx compile it to CommonJS, which needs an ESM-to-CommonJS transform oxc does not have.
- **Embedding** ([oxc-node#804](https://github.com/oxc-project/oxc-node/issues/804), [oxc-node#805](https://github.com/oxc-project/oxc-node/issues/805), [oxc-node#806](https://github.com/oxc-project/oxc-node/issues/806), v0.1.6): a `default_global_allocator` feature, a tracing subscriber installed only when `OXC_LOG` is set and never over an existing one, and `setTsconfigPath()` for an explicit tsconfig without environment variables. See [Native Hooks in the Binding](#3-native-hooks-in-the-binding).

### Fixed in the Loader JavaScript

1. **Runtime helper resolution.** Lowered code imports helpers such as `@oxc-node/core/helpers/defineProperty`, which would resolve from the user's file, where `@oxc-node/core` is usually not installed (`ERR_MODULE_NOT_FOUND`). The loader maps them to `@oxc-project/runtime`, a `vite-plus` dependency, resolved from `vite-plus` itself: the ESM resolve hook calls `nextResolve('@oxc-project/runtime/helpers/<name>', { parentURL: <vite-plus dist URL> })`.
2. **Helpers in CommonJS output.** Output without `import`/`export` loads helpers with `require("@oxc-node/core/helpers/…")`, which the off-thread `module.register()` hooks never see. The pirates hook rewrites each to `process.getBuiltinModule("node:module").createRequire(<vite-plus dist URL>)("@oxc-project/runtime/helpers/<name>")`, so it resolves from `vite-plus`.

### Fixed in the Vendored Source

`packages/tools/patches/oxc-node.patch` (see [Native Hooks in the Binding](#3-native-hooks-in-the-binding)) carries one behavior change, marked `Vite+:` in the source:

3. **TypeScript under `node_modules`** is transformed (`.ts .mts .cts .tsx` only; other dependency files run as published). Node.js refuses to strip types there (`ERR_UNSUPPORTED_NODE_MODULES_TYPE_STRIPPING`). Upstream transforms dependencies only under `OXC_TRANSFORM_ALL`, which also transforms their JavaScript; an option for TypeScript dependencies alone is requested in [oxc-node#821](https://github.com/oxc-project/oxc-node/issues/821).

### Blocked on Oxc

- TC39 standard decorators.
- ESM-to-CommonJS output, so a `.cts` with `import`/`export` could compile to CommonJS, as with tsc and tsx, and `require()` of a CommonJS-scoped `.ts` file with ESM syntax could stay CommonJS. Today the first is an error, and the second runs as an ES module, where `require` and `module` are not defined.

### Not Planned

- Yarn PnP. oxc-node never turns PnP on in its `ResolveOptions`, and Vite+ does not need it for script mode.

### Known Limitations

- tsconfig `paths` apply to `import` only. `require()` uses Node.js' CommonJS resolution, as upstream.
- A tsconfig `module` of `node16`, `node18`, or `nodenext` makes `.ts` and `.tsx` files ES modules even in a package without `"type": "module"`, as upstream does. `tsc` and tsx follow `package.json` `"type"` for those values.

### Nice to Have

- Transform TypeScript in `--eval`, stdin, and the REPL.
- An opt-in persistent transform cache.
- TypeScript syntax in extensionless entry files, for shebang scripts without `.ts`.
- Split the crate into a NAPI-free core and a thin NAPI layer, so the vendored patch shrinks ([oxc-node#807](https://github.com/oxc-project/oxc-node/issues/807)).

## Implementation Architecture

### 1. Detection and Dispatch

**Files**: `crates/vp_global_cli/src/commands/vpx.rs`, `crates/vp_global_cli/src/commands/vpx_script.rs`

- `parse_vpx_args` accepts `--tsconfig <path>`, `--tsconfig=<path>`, and `-v/--version`. `vpx --version` prints the same report as `vp --version`.
- `vpx_script::detect(&positional, cwd) -> Result<Option<ScriptInvocation>, ScriptError>` runs in `execute_vpx` before `extract_command_name` and `has_version_spec`. `None` means package mode. A `ScriptInvocation` carries the Node.js arguments with any `--tsconfig` among the Node.js options removed.
- `vpx_script::execute` resolves the loader directory and sets or clears `VP_SCRIPT_TSCONFIG`. It prepends `node_modules/.bin`, then calls the core `node` shim dispatch with `["--require", preload_path, "--import", register_url, ...node_args]`. The call is boxed, because shim dispatch is also what routes `vpx` there.

The detection rules are pure functions over `(tokens, cwd)` and are unit tested in the same module.

### 1b. Project-Local `vpx` Bin

**Files**: `packages/cli/bin/vpx`, `packages/cli/src/vpx-bin.ts`, `packages/cli/src/vpx-script.ts`, `packages/cli/package.json`

- `"vpx": "./bin/vpx"` sits next to `vp` and `vpr`. The shim imports `dist/vpx-bin.js`, a separate entry that loads neither the CLI bundle nor the native binding; the script's own Node.js process loads the binding through the preload.
- Script mode: the same detection rules (`vpx-script.ts`), then `spawn(process.execPath, ['--require', preload, '--import', registerUrl, ...nodeArgs], { stdio: 'inherit' })`, with the signal handling described in [Process Model](#process-model).
- Package mode, `--help`, and `--version` go to the global `vpx` found on `PATH`, skipping `node_modules/.bin` directories. Without one, package mode prints that running package binaries needs the global CLI, and `--version` prints this package's version. Inside `vp run` and package manager scripts, where `node_modules/.bin/vpx` shadows the global shim, `vpx eslint .` therefore behaves as before.
- A unit test checks that the Rust and TypeScript lists of script extensions and value-taking Node.js options stay identical.

### 2. Loader Resolution

**File**: `crates/vp_global_cli/src/commands/vpx_script.rs`

`resolve_loader_dir(cwd)` tries the project's `vite-plus` package and then the global scripts dir, reusing `JsExecutor::resolve_local_vite_plus_package_dir` and `JsExecutor::get_scripts_dir`. A project `vite-plus` too old to ship both entries falls back to the global one.

### 3. Native Hooks in the Binding

**Files**: `packages/tools/.upstream-versions.json`, `packages/tools/src/sync-remote-deps.ts`, `packages/tools/src/patch-oxc-node.ts`, `packages/tools/patches/oxc-node.patch`, `.github/actions/clone/action.yml`, `.github/workflows/security.yml`, `.github/scripts/upgrade-deps.ts`, `Cargo.toml`, `packages/cli/binding/Cargo.toml`, `packages/cli/binding/src/lib.rs`, `packages/cli/build.ts`

oxc-node is vendored the way Rolldown is:

- `.upstream-versions.json` pins oxc-node at its latest release tag (v0.1.6), and the daily `upgrade-deps` workflow bumps it to new stable tags, as it does for Rolldown and Vite.
- `sync-remote` clones it into the gitignored `oxc-node/` and runs `patch-oxc-node.ts` after the Cargo oxc sync. CI checks it out in the shared clone action and the security workflow, and runs the same script before any cargo command, because `Cargo.lock` records the patched manifest. The script and the patch are part of the native cache key.
- `patch-oxc-node.ts` applies `patches/oxc-node.patch` with `git apply` (a reverse check makes it idempotent) and then points the `oxc` dependency at the workspace version. After editing `oxc-node/` by hand, `pnpm tool patch-oxc-node --update` writes the changes back to the patch, leaving the oxc version out so the patch keeps applying.
- The root `Cargo.toml` adds `oxc-node = { path = "./oxc-node", features = ["default_global_allocator"] }`, leaving the global allocator to `rolldown_binding`, and excludes `oxc-node/` from the workspace, so upstream code is not held to `--deny warnings` workspace lints, like the nested rolldown workspace.
- The binding has an optional `oxc-node` feature and does `pub extern crate oxc_node;` next to `rolldown_binding`. `build.ts` enables `rolldown,oxc-node` for every build; napi-rs passes each `features` entry as a separate cargo argument, so the list is joined into one.

What the binding already contains makes this cheap. Through `rolldown_binding`, the binding links the same crates oxc-node uses (`oxc` with transformer, codegen, and semantic; `oxc_resolver` 11.24.3; `oxc_sourcemap` 9). oxc-node's `src/lib.rs` (about 2,070 lines with the patch) and `src/windows_file_url.rs` (about 1,170 lines) are glue over those crates.

Besides [Fixed in the Vendored Source](#fixed-in-the-vendored-source), the patch adapts the crate to its host:

| Change                                                                                                      | Reason                                                                                                                                                                                                                             |
| ----------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `crate-type = ["rlib"]` and `build = false`                                                                 | Linked into the host cdylib, whose build script already sets the napi link args                                                                                                                                                    |
| Every top-level `#[napi]` item goes under `namespace = "oxcNode"`; `TransformTask` → `OxcNodeTransformTask` | `rolldown_binding` links `oxc_transform_napi`, which also exports `transform` and has a `TransformTask`; duplicate NAPI exports silently overwrite each other ([oxc-node#807](https://github.com/oxc-project/oxc-node/issues/807)) |
| `oxc` follows the workspace version (applied by the script, not the patch)                                  | One copy of oxc: oxc-node tracks oxc head while Rolldown and Vite+ can lag; both use 0.153.0 today                                                                                                                                 |

Everything else uses upstream options: the `default_global_allocator` feature, the tracing subscriber that stays off unless `OXC_LOG` is set (so `VP_LOG` keeps its own), and `setTsconfigPath()`, which the preload calls with `VP_SCRIPT_TSCONFIG`.

Before this change, `BUNDLING.md` said the `rolldown` feature was release-only, but `build.ts` enables it for every build, so development builds already compile oxc. The `oxc-node` feature costs dev builds no extra crates.

### 3b. Loader JavaScript

**Files**: `packages/cli/src/script-preload.ts`, `packages/cli/src/script-register.ts`, `packages/cli/src/script-esm-hooks.ts`, `packages/cli/src/script-hooks.ts`, `packages/cli/package.json`, `packages/cli/tsdown.config.ts`

- The entries are adapted from oxc-node's `packages/core/register.mjs` (160 lines) and `esm.mjs` (26 lines). They use `oxcNode` from the Vite+ binding instead of `@oxc-node/core`, and add the Node.js version check and the helper mapping (`script-hooks.ts`). They sit at the top of `src/` because the bundle flattens into `dist/` and the binding import stays relative.
- `script-preload.ts` is built by its own tsdown block as one CommonJS file, `dist/script-preload.cjs`, with `import.meta.url` shimmed.
- `pirates` is a `devDependency`, bundled into `dist`.
- `@oxc-project/runtime` is a `dependency` of `vite-plus`, so helpers resolve from `vite-plus` under strict layouts such as pnpm. Its catalog pin follows Rolldown's (`sync-remote` no longer takes the higher version, and `upgrade-deps` no longer bumps it to the latest npm release), so it matches the oxc crates compiled into the binding: 0.153.0 today.

### 4. Toolchain Metadata and Docs

- `packages/cli/toolchain.config.json` registers `oxc-node` with a new `vendored` version source: the version comes from the vendored `@oxc-node/core` package.json and the revision from `.upstream-versions.json`. `vp toolchain` shows `compiles oxc-node@0.1.5 (<revision>)`, which compiles `oxc` and `oxc-resolver`.
- `packages/cli/BUNDLING.md`: an "oxc-node Script Loader" section, plus the corrected feature description.
- `docs/guide/vpx.md`: separate "Running Package Binaries" and "Running Scripts" sections, with the tsconfig defaults. `docs/guide/env.md` contrasts `vp node` with `vpx <file>`.
- The `vpx --help` text (`VPX_HELP`) and the `command_vpx_pnpm*` snapshots that record it.

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
error: vpx: Script not found: ./scripts/missing.ts

$ vpx -p cowsay ./a.ts
error: vpx: --package cannot be used when running a script

$ vpx --tsconfig tsconfig.missing.json ./a.ts
error: vpx: tsconfig not found: tsconfig.missing.json

$ vpx ./a.ts          # engines.node pins 20.18.0
error: Running scripts with vpx requires Node.js ^22.18.0 || ^24.11.0 || >=26.0.0 (current: v20.18.0)

$ vpx ./decorators.ts # no experimentalDecorators
Error: Failed to transform file:///…/decorators.ts: decorators require `"experimentalDecorators": true` in a tsconfig.json that includes this file; standard (TC39) decorators are not supported yet
```

Other errors raised in the script, including transform errors, come from Node.js and oxc-node unchanged. Source maps are on, so stack traces point at the TypeScript source.

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

### 4. Why Vite+-Owned Loader Entries

**Decision**: Pass `--require <vite-plus>/dist/script-preload.cjs --import <vite-plus>/dist/script-register.js` instead of `--import @oxc-node/core/register`.

**Rationale**:

- It provides one place for the Node.js version check, the helper specifier rewrite, and binding loading.
- The CommonJS `--require` preload registers the hooks before any user `--require`, so `--require ./setup.ts` works; tsx splits its loader the same way (`--require preflight.cjs --import loader.mjs`). A single ESM entry passed to both flags does not work: on Node.js 22, `module.register()` cannot run while a `--require` loads an ES module.
- The `--import` routes the entry point through the ESM loader, where the hooks decide each file's format; a CommonJS-only start would compile a `.ts` entry with `import` syntax as CommonJS.
- The oxc-node version follows the `vite-plus` version, which the project pins.
- The hooks implementation can change without changing the CLI contract.
- It can later become a public `node --import vite-plus/register` entry (see [Future Enhancements](#future-enhancements)).

### 5. Why the Loader Goes in argv Instead of `NODE_OPTIONS`

**Decision**: Pass the loader as an argv flag.

**Rationale**: `process.execArgv` carries it into `fork()` and worker threads, which may load TypeScript. Unrelated Node.js processes the script spawns, such as package managers and `vp` itself, do not load it. tsx behaves the same way.

### 6. Why Explicit Paths Never Download

**Decision**: An explicit path, or a bare name with a TypeScript extension, is never treated as a package spec.

**Rationale**: a mistyped path such as `vpx ./sed.ts` (for `./seed.ts`) should report a missing file. Today it can fall through to `vp dlx`, which runs remote code under an unexpected name.

### 7. Why Resolve the Loader From the Project First

**Decision**: Use the project's `vite-plus` when its `dist/` contains both loader entries, and fall back to the global install.

**Rationale**: the global CLI already delegates JS entry points this way. Teams and CI then run the hooks and binding of the `vite-plus` version pinned in the lockfile. The cost is the version-skew contract described under [Loader Entries](#loader-entries).

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
- The blocking helper bug stops being an upstream dependency. The loader maps helper imports to `@oxc-project/runtime`, a `vite-plus` dependency, and the vendored copy carries fixes such as enum evaluation until they land upstream.
- One oxc version for the whole toolchain. Rolldown, `vp lint`'s type-aware parsing, the static config extractor, and the script loader cannot drift apart.
- The vendoring mechanism (`sync-remote`, `.upstream-versions.json`, workspace path deps, a Cargo feature, publish-time platform injection) exists and is exercised on every release.

**Costs**: oxc-node tracks oxc head and bumps often (18 oxc updates in its changelog), so a new tag must compile against the workspace's oxc version, and the vendored copy carries `patches/oxc-node.patch` until its fixes land upstream. A bump whose patch no longer applies fails `sync-remote` loudly and needs `pnpm tool patch-oxc-node --update` after rebasing the changes.

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
  - Helper resolution (Fixed in the Loader item 1) would still need the same loader-side mapping, plus coordination with a separately versioned package.
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

An extensionless executable with that shebang (`bin/tool`) does not work for TypeScript in v1. Node.js treats extensionless files as JavaScript, and oxc-node's hooks only claim known extensions, so the file runs as plain JavaScript (see Nice to Have). The shebang check in `is_script` exists only to stop a loop: without it, `vpx /abs/bin/tool` would go through the `PATH` lookup, exec the file, and re-enter `vpx` forever. Options in a shebang need `#!/usr/bin/env -S vpx --tsconfig …`.

### Scripts Outside the Current Directory

The Node.js runtime and `node_modules/.bin` are resolved from the cwd, matching `vp node`. Imports resolve from the script's location (Node.js semantics), and each file uses its nearest tsconfig.

### `--require` Preloads

Node.js runs every `--require` before any `--import`. The loader's own `--require` comes first in argv, so a user's `--require ./setup.ts` runs after the CommonJS hook is installed, and `--import ./setup.ts` after all hooks are registered. Both preloads can be TypeScript.

### `.cts` Files With ESM Syntax

TypeScript and tsx compile `import`/`export` in a `.cts` file to CommonJS. oxc cannot, so such a file fails with one error on every load path, which suggests renaming it to `.mts` or writing it with `import x = require()` and `export =`. Type-only imports and exports are erased first and allowed. Node.js' own type stripping rejects the file too ([oxc-node#811](https://github.com/oxc-project/oxc-node/issues/811)).

### `TS_NODE_PROJECT` and `OXC_TSCONFIG_PATH`

oxc-node reads both when no path is set through `setTsconfigPath()`, with `TS_NODE_PROJECT` taking precedence. `vpx --tsconfig` sets one, so it always wins. Without the flag, a value left over from ts-node changes tsconfig selection, as it does for `oxnode`; the guide documents this.

### Windows

There are no shebangs. `vpx scripts\a.ts` works through the file-exists rule; the preload is passed as a path and the `--import` as a `file:///C:/…` URL. Exit codes and Ctrl+C follow the existing `node` shim spawn-and-wait path. CI runs the snapshot cases on Windows, except watch mode and shebangs.

### Inside `vp run` and `package.json` Scripts

With the global CLI installed, `node_modules/.bin/vpx` (the project-local bin) shadows the global shim on `PATH` inside `vp run` and package manager scripts. Both resolve the same loader and the same Node.js, so the difference is the extra parent process of the local bin. Without the global CLI, only the local bin exists and package mode prints the existing "requires the global CLI" hint.

## Testing Strategy

### Unit Tests

- `crates/vp_global_cli/src/commands/vpx_script.rs` and `vpx.rs`: the detection table, option-value skipping, `--tsconfig` extraction, the shebang guard, and `-v/--version`.
- `packages/cli/src/__tests__/vpx-script.spec.ts`: the TypeScript port of the same rules; a check that the Rust and TypeScript lists match; the Node.js range check, which must equal `engines.node`; the sync-hooks cutoff; and the helper mapping and `require()` rewrite.
- `packages/tools/src/__tests__/patch-oxc-node.spec.ts`: the oxc version rewrite, the patch's scope, and that it is applied to the vendored checkout.
- `packages/tools/src/__tests__/sync-remote-deps.spec.ts`: `@oxc-project/runtime` follows Rolldown's pin.
- `packages/cli/src/__tests__/toolchain.spec.ts`: the `oxc-node` toolchain node with its version and revision.

### Snapshot Tests

Every case runs in both flavors (`vp = ["local", "global"]`), and the global shim and the project-local bin record identical snapshots. The cases need a built `packages/cli` (dist and binding) in both flavors, because script mode loads the loader entries and the binding from the installed checkout package.

`fixtures/command_vpx_script/` pins Node.js 22.18.0, the repository version CI prewarms, so it covers the `module.register()` path:

| Case                             | Covers                                                                                                                                                        |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `vpx_script_ts_syntax`           | Enum, namespace, parameter properties, legacy decorator, private field, lowered class field (helper)                                                          |
| `vpx_script_resolution`          | tsconfig `paths`, `.js` → `.ts`, extensionless import, JSON, `require()` of a CommonJS `.cts` (`export =`) with a class field; `--tsconfig` in both positions |
| `vpx_script_args_and_options`    | Arguments and `--` after the script, exit code 7, `--env-file .env`, a TypeScript `--import` preload, `--eval` without a script                               |
| `vpx_script_errors`              | Missing explicit path, missing bare `.ts`, missing tsconfig, `-p` with a script                                                                               |
| `vpx_script_module_formats`      | The `.cts` module-syntax error as the entry and when imported, a TypeScript `--require` preload, `fork()` and `Worker`, `--test`                              |
| `vpx_script_ts_dependency`       | A TypeScript package in `node_modules`                                                                                                                        |
| `vpx_script_tsconfig_transforms` | Classic and automatic JSX runtimes, native class fields, `verbatimModuleSyntax`, the decorator error                                                          |
| `vpx_script_shebang` (Unix)      | An executable `.ts` file and an extensionless file with `#!/usr/bin/env vpx`                                                                                  |
| `vpx_script_watch` (Unix)        | `--watch` restarts when an imported `.ts` file changes (driven by `verify-watch.mjs`)                                                                         |

`fixtures/command_vpx_script_node24/` pins Node.js 24.12.0 for the `registerHooks()` path on an LTS line, including an imported CommonJS module that `require()`s another file. `fixtures/command_vpx_script_node26/` pins Node.js 26.5.0 for the same path (ESM, `require()` of CommonJS `.cts`, native `using`, the `.cts` module-syntax error). `fixtures/command_vpx_script_old_node/` pins Node.js 20.18.0 for the unsupported-version error. Both download their runtime in CI.

`command_vpx_pnpm10` and `command_vpx_pnpm11` re-record the new help text; package mode is otherwise unchanged. `command_toolchain` re-records the `oxc-node` node.

### Prototype Verification Checklist

Verified on macOS arm64 with Node.js 22.18, 24.11, 26.0, and 26.5, through the project-local bin (and the global shim on 22.18):

- [x] The vendored crate compiles against the workspace's oxc (0.152.0, then 0.153.0) without source changes, and `Cargo.lock` holds one `oxc`.
- [x] The `oxcNode` namespace removes the export collisions; Rolldown's `transform` and `TransformTask` are unchanged.
- [x] Helpers resolve for class fields from ESM, from CommonJS, and from a `require()`d CommonJS `.cts` file.
- [x] A `.cts` with `export` fails with the same error as the entry and when imported, on both hook paths.
- [x] `VP_SCRIPT_TSCONFIG` drives both resolution (`paths`) and per-process tsconfig selection.
- [x] The binding loads inside the `module.register()` worker thread.
- [x] `--watch` restarts when an imported `.ts` file changes, on both hook paths (22.18 and 26.5).
- [x] `--test` with `.ts` test files; `fork('./child.ts')` and `new Worker('./worker.ts')`.
- [x] A TypeScript `--require` preload; TypeScript in `node_modules`; export conditions after a CommonJS entry.
- [x] No DEP0205 warning on Node.js 26.0 and 26.5.
- [x] Exit codes, and `128 + signal` through the project-local bin (SIGTERM gives 143).
- [x] Release-build binary size growth (see [Performance](#performance)).

Still to verify:

- [ ] Windows: paths, loader URL, Ctrl+C, and exit codes (CI snapshot run).

## Performance

Startup compared with tsx 4.23.15 on an Apple M4 Pro (12 cores, macOS 26.6), at commit `460f320a`. `bench/vpx-script.ts` produces the table below. To rerun it, build the loader with `pnpm --filter vite-plus build-ts`, then run `node bench/vpx-script.ts --node <node> [--node <node>…]` with one `--node` per Node.js binary to measure.

The script generates the scenarios in the OS temp directory, outside any `tsconfig.json`, and checks that every command prints the same output without warnings. It then reports the median wall time of the whole command over 20 runs, after 3 warmup runs.

The scenarios:

- `hello`: one `.ts` file with erasable syntax only, so Node.js type stripping can run it too.
- `esm-app`: an `index.ts` that imports 300 modules, each with an interface, an enum, and a class with a field and a parameter property, in a `"type": "module"` package.
- `cjs-app`: the same files in a package without `"type"`.
- `large-cjs`: one 1.8 MB `.ts` file without `import`/`export`, in a package without `"type"`.
- `large-esm`: the same file in a `"type": "module"` package.

The commands:

- `vpx` loader: `node --require dist/script-preload.cjs --import dist/script-register.js`, which is what the global `vpx` execs in place. The Rust shim's own startup is not included.
- `node --import tsx`: tsx's loader in the same process, for comparison.
- `vpx` local bin and the `tsx` CLI: both start the script in a child Node.js process.
- tsx caches transforms on disk by default; "no cache" sets `TSX_DISABLE_CACHE=1`. `vpx` has no transform cache.

On Node.js 22.18 both loaders run their ESM hooks on a `module.register()` thread; on 24.12 and 26.5 both run them in-thread.

| Scenario    | Command                         | Node.js 22.18.0 | Node.js 24.12.0 | Node.js 26.5.0 |
| ----------- | ------------------------------- | --------------: | --------------: | -------------: |
| `hello`     | `vpx` loader (1 process)        |           50 ms |           26 ms |          30 ms |
| `hello`     | `node --import tsx` (1 process) |           68 ms |           46 ms |          41 ms |
| `hello`     | `node --import tsx`, no cache   |           75 ms |           63 ms |          62 ms |
| `hello`     | `vpx` local bin (2 processes)   |           69 ms |           49 ms |          55 ms |
| `hello`     | `tsx` CLI (2 processes)         |           98 ms |           81 ms |          72 ms |
| `hello`     | `tsx` CLI, no cache             |          106 ms |           95 ms |          94 ms |
| `hello`     | `node` type stripping           |           48 ms |           44 ms |          48 ms |
| `esm-app`   | `vpx` loader (1 process)        |           87 ms |           65 ms |          70 ms |
| `esm-app`   | `node --import tsx` (1 process) |          109 ms |           81 ms |          84 ms |
| `esm-app`   | `node --import tsx`, no cache   |          140 ms |          217 ms |         216 ms |
| `esm-app`   | `vpx` local bin (2 processes)   |          113 ms |           87 ms |          92 ms |
| `esm-app`   | `tsx` CLI (2 processes)         |          143 ms |          114 ms |         113 ms |
| `esm-app`   | `tsx` CLI, no cache             |          173 ms |          250 ms |         251 ms |
| `cjs-app`   | `vpx` loader (1 process)        |           87 ms |           65 ms |          67 ms |
| `cjs-app`   | `node --import tsx` (1 process) |          127 ms |          100 ms |          95 ms |
| `cjs-app`   | `node --import tsx`, no cache   |          289 ms |          261 ms |         258 ms |
| `cjs-app`   | `vpx` local bin (2 processes)   |          108 ms |           89 ms |          93 ms |
| `cjs-app`   | `tsx` CLI (2 processes)         |          160 ms |          132 ms |         128 ms |
| `cjs-app`   | `tsx` CLI, no cache             |          325 ms |          293 ms |         291 ms |
| `large-cjs` | `vpx` loader (1 process)        |          115 ms |          103 ms |          95 ms |
| `large-cjs` | `node --import tsx` (1 process) |          122 ms |           90 ms |          76 ms |
| `large-cjs` | `node --import tsx`, no cache   |          226 ms |          274 ms |         258 ms |
| `large-cjs` | `vpx` local bin (2 processes)   |          136 ms |          125 ms |         126 ms |
| `large-cjs` | `tsx` CLI (2 processes)         |          153 ms |          123 ms |         110 ms |
| `large-cjs` | `tsx` CLI, no cache             |          262 ms |          303 ms |         294 ms |
| `large-esm` | `vpx` loader (1 process)        |           97 ms |           77 ms |          79 ms |
| `large-esm` | `node --import tsx` (1 process) |          106 ms |           75 ms |          70 ms |
| `large-esm` | `node --import tsx`, no cache   |          200 ms |          178 ms |         173 ms |
| `large-esm` | `vpx` local bin (2 processes)   |          118 ms |           99 ms |         104 ms |
| `large-esm` | `tsx` CLI (2 processes)         |          137 ms |          107 ms |         103 ms |
| `large-esm` | `tsx` CLI, no cache             |          230 ms |          209 ms |         206 ms |

What the numbers show:

- **`vpx` against tsx with a warm cache.** For one process, the `vpx` loader is 1.2–1.8× faster on `hello`, `esm-app`, and `cjs-app` on every Node.js version. On 24.12, for example, `hello` takes 26 ms against 46 ms and `esm-app` 65 ms against 81 ms.
- **Against a cold or disabled tsx cache**, `vpx` is 1.5–4.0× faster in every scenario.
- **Against Node.js type stripping**, the `vpx` loader is on par on 22.18 (50 against 48 ms) and faster on 24.12 and 26.5 (26–30 against 44–48 ms).
- **Megabyte-sized files are where tsx's cache pays off**, because it skips the transform while `vpx` transforms 1.8 MB on every run.
  - `large-esm`: about even. `vpx` is faster on 22.18 (97 against 106 ms); tsx is 2 ms faster on 24.12 and 9 ms faster on 26.5.
  - `large-cjs`: tsx is 13–19 ms faster on 24.12 and 26.5. For `vpx`, the CommonJS file costs 17–26 ms more than the same file as an ES module: the ESM load hook reads and parses it once more to check for ES module syntax, and on the in-thread path it is also transformed twice ([oxc-node#820](https://github.com/oxc-project/oxc-node/issues/820)).
- **The local bin costs 19–30 ms** for its second Node.js process. The `tsx` CLI pays the same, so local bin against CLI keeps roughly the same ratios.

A persistent transform cache would therefore only help with megabyte-sized sources, and stays under [Nice to Have](#nice-to-have). Fixing oxc-node#820 removes most of the CommonJS gap.

Binding load cost, measured by `require()` of the `.node` file alone on Node.js 22.18, macOS arm64:

| Binding                               | Size   | Warm load  | Cold load |
| ------------------------------------- | ------ | ---------- | --------- |
| `vite-plus.darwin-arm64.node` (1.1.0) | 42 MB  | 2.3–3.2 ms | 17.9 ms   |
| `oxc-node.darwin-arm64.node` (0.1.3)  | 4.2 MB | 0.7–1.1 ms | 736 ms    |

The 2 ms difference is within the noise of the startup numbers above, so compiling the hooks into the Vite+ binding costs nothing a user can notice. The cold numbers are first-touch page cache effects, not representative.

Release-build size of the binding on macOS arm64 (`cargo build --release -p vite-plus-cli`, fat LTO):

| Features            | `libvite_plus_cli.dylib` |
| ------------------- | ------------------------ |
| `rolldown`          | 42,016,656 bytes         |
| `rolldown,oxc-node` | 42,149,328 bytes         |

Compiling oxc-node in adds about 130 KB (0.3%), against about 4.2 MB for a separate `@oxc-node/core` platform package.

## Security Considerations

1. **No remote fallback for files**: explicit paths, and bare names with TypeScript extensions, never reach `vp dlx`.
2. **Trusted loader**: the hooks are compiled into the `vite-plus` binding and its `dist`; runtime helpers resolve from `vite-plus`'s own `@oxc-project/runtime` dependency, never from the project's `node_modules`.
3. **Shebang recursion guard**: prevents an exec loop for extensionless `#!/usr/bin/env vpx` files.
4. **No new environment surface**: `VP_SCRIPT_TSCONFIG` is set only when `--tsconfig` is passed (and cleared otherwise), and reaches oxc-node through `setTsconfigPath()`, not through `TS_NODE_PROJECT` or `OXC_TSCONFIG_PATH`, which other tools read.

## Backward Compatibility

Package mode is unchanged. Behavior changes only for invocations that run a file:

- `vpx <existing file with a script extension>` now runs the file. Previously an executable file with a shebang was exec'd through the `PATH` lookup, and a non-executable file fell through to `vp dlx`, which failed.
- A bare name that matches both a local file and a package (`vpx highlight.js` with `./highlight.js` present) now runs the file, as `node` and `tsx` would.
- Invocations whose first token after the `vpx` options starts with `-` previously reached `vp dlx` with that token as the package name and never ran a package. They now run Node.js. `vpx --version` previously printed the package manager's version through `pnpm dlx --version`; it now prints the Vite+ version, like `vp --version`.
- `vite-plus` gains a `vpx` bin. Projects that already have a different `vpx` in `node_modules/.bin` (there is no such package on npm today) would see a bin conflict warning from their package manager.
- No new platform packages; see [Performance](#performance) for the binding size. `vite-plus` gains `@oxc-project/runtime` (pure JavaScript helpers) as a dependency; `pirates` is bundled. The `@oxc-project/runtime` catalog pin follows Rolldown's, which `@voidzero-dev/vite-plus-core` also uses.

## Rollout

1. **Phase 0, vendor and prototype** (implemented): oxc-node in `sync-remote`, CI, and `upgrade-deps`; the patch; the binding feature; the loader entries; detection and execution in both CLIs; toolchain metadata; unit and snapshot tests; and docs.
2. **Phase 1, release** (implemented, pending review): the feature ships as a regular part of `vpx`. CI covers Linux, macOS, and Windows, and the upstream fixes from [Upstream Status](#upstream-status-and-vendored-changes) landed in oxc-node v0.1.5 and v0.1.6.
3. **Phase 2, migration**: teach `vp migrate` to rewrite `tsx`, `ts-node`, and `esno` usages in `package.json` scripts.
4. **Phase 3**: see Future Enhancements.

## Open Questions

1. **Command surface**: is `vpx <script>` enough, or should `vp exec <script>` also run scripts? Should `vp node` stay plain Node.js? (Proposed: `vpx` only, and `vp node` stays plain.)
2. **Bare names**: should `vpx seed.ts`, without `./`, run an existing file? Should a missing bare `foo.ts` be an error or a package lookup? (Proposed: run the file, and report an error.)
3. **JavaScript files**: should `.js`, `.mjs`, `.cjs`, and `.jsx` also get the loader, for example so that `.js` can import `.ts`? (Proposed: yes.)
4. **Node.js range**: should script mode require the Vite+ engines range (`^22.18.0 || ^24.11.0 || >=26.0.0`), or anything with `module.register()` (≥ 20.6)? Node.js 20 reached end of life in April 2026. (Proposed: the engines range.)
5. **Loader source**: project `vite-plus` first, or always the global install? (Proposed: project first.)
6. **Local bin scope**: Design Decision 8 ships a project-local `vpx` that handles script mode and delegates package mode. Should it instead implement the full lookup chain in JS, so that `vpx eslint .` also works without the global CLI? (Proposed: delegate; keep one implementation of the chain.)
7. **`--require` ordering** (resolved): the loader is split into a CommonJS `--require` preload and an `--import`, so `--require ./setup.ts` works.
8. **Extensionless TypeScript in shebang scripts**: wait for upstream support, or document the `.ts` requirement? (Proposed: document; see Edge Cases.)
9. **oxc version lockstep** (resolved): `upgrade-deps` bumps oxc-node to its latest stable tag, and `sync-remote` rewrites its `oxc` version to the workspace's and re-applies the patch; a tag that does not compile or patch fails the upgrade loudly. `@oxc-project/runtime` follows Rolldown's pin.
10. **Maturity** (resolved): the docs present script mode as a regular `vpx` feature, without an experimental label. The oxc.rs docs still mark oxc-node itself as experimental; the vendored copy is pinned to a release and covered by Vite+'s own tests.
11. **`--version`** (resolved): `vpx -v/--version` prints the Vite+ version, as `vp --version` does; `vp node --version` prints the Node.js version.
12. **Dev builds** (resolved): `build.ts` already compiles Rolldown, and therefore oxc, into every build, so the `oxc-node` feature is on for all builds at no extra crate cost.
13. **`.cts` files with ESM syntax** (resolved): follow upstream, which reports one error on every load path ([oxc-node#811](https://github.com/oxc-project/oxc-node/issues/811), v0.1.6). Running them as tsx does needs an ESM-to-CommonJS transform in oxc.

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
