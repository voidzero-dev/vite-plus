# RFC: Run TypeScript Scripts with `vpx`

- Status: Implemented in [#2936](https://github.com/voidzero-dev/vite-plus/pull/2936); CI covers Linux, macOS, and Windows on Node.js 22, 24, and 26
- Related: [`vpx` command](./vpx-command.md), [JavaScript runtime management](./js-runtime.md), [CLI bundling](../packages/cli/BUNDLING.md), [Core binding resolution](./core-binding-resolution.md)
- Upstream: [oxc-node](https://github.com/oxc-project/oxc-node) v0.1.7, vendored unpatched and compiled into the Vite+ native binding, as Rolldown is
- Prior art: [tsx](https://github.com/privatenumber/tsx) 4.23.15

## Summary

`vpx` runs a script file directly:

```bash
vpx ./scripts/seed.ts --dry-run
```

When the command names a script, `vpx` runs it on the project's managed Node.js with the oxc-node loader preloaded, instead of resolving a package binary:

```
vpx ./seed.ts args  ──►  node --require …/vite-plus/dist/script-preload.cjs
                              --import file:///…/vite-plus/dist/script-register.js ./seed.ts args
```

The loader's native half is compiled into the Vite+ binding; its JavaScript half ships in `vite-plus/dist`. TypeScript syntax that Node.js cannot strip (enums, namespaces, parameter properties, legacy decorators, JSX) works without a `tsx` dependency, as do tsconfig `paths`, `.js` → `.ts` imports, TypeScript dependencies, and ESM/CJS interop. Package-binary execution (`vpx eslint .`) is unchanged. A project-local `vpx` bin makes it work in `package.json` scripts without the global CLI.

## Motivation

Repositories use `tsx` to run `scripts/*.ts`. Vite+ manages the Node.js version and ships the rest of the toolchain, but running a TypeScript file meant adding `tsx` (esbuild, two processes), downloading it through `vpx tsx`, or settling for `vp node` and Node.js type stripping, which only erases syntax: no enums, decorators, JSX, tsconfig `paths`, or TypeScript in `node_modules`. VoidZero maintains oxc-node, an Oxc-based loader that this repository and Rolldown already use for their own scripts. Exposing it through `vpx` gives users a tsx equivalent with no extra dependency, on the project-pinned Node.js.

## Goals

1. `vpx <script>` runs `.ts .mts .cts .tsx .js .mjs .cjs .jsx` files with full TypeScript syntax, tsconfig `paths`, `.js` → `.ts` imports, and both module systems.
2. Scripts run on the Node.js that `vp node` would select; the project-local bin runs them on the Node.js that started it.
3. On Unix, one process: exit codes, signals, stdio, and `process.ppid` behave as with `node`.
4. Node.js options (`--watch`, `--inspect`, `--env-file=`, `--test`, `--import`) are forwarded.
5. `#!/usr/bin/env vpx` works for files that keep a script extension.
6. The loader ships inside `vite-plus`: no new npm package, no user dependency.
7. `vpx <script>` works from a project-local `vite-plus` without the global CLI.
8. Package-binary behavior of the global `vpx` is unchanged.

## Non-Goals

Type checking (`vp check`); a TypeScript REPL or TypeScript in `-e`/`-p`; a tsx-style `watch` subcommand (use Node.js `--watch` and `--watch-path`); a programmatic API or public `node --import vite-plus/…` entry (see [Future Enhancements](#future-enhancements)); an on-disk transform cache; `target` downleveling; Bun and Deno.

## Command Syntax

```bash
vpx [VPX_OPTIONS] [NODE_OPTIONS] <script> [args...]
```

- `VPX_OPTIONS`: the existing `vpx` options plus `--tsconfig <path>` and `-v/--version`. `-p/--package` and `-c/--shell-mode` select package mode, so combining them with a script is an error. `-s/--silent` is accepted; script mode prints no Vite+ output either way.
- `NODE_OPTIONS`: tokens starting with `-` between the `vpx` options and the script, forwarded to Node.js verbatim. `--tsconfig` is also recognized here (`vpx --watch --tsconfig x.json ./a.ts`).
- `<script>` and `[args...]`: passed to Node.js unchanged. Tokens after the script, including `--`, belong to the script.

`vpx` options are parsed first, so `-p`, `-c`, `-h`, and `-v` keep their `vpx` meaning; Node.js has the long forms, and `vp node --version` prints the Node.js version. `vpx --version` prints the same report as `vp --version`.

When the first positional token starts with `-` and no script follows, `vpx` forwards the arguments to Node.js with the loader: `--eval`, `--print`, `--check ./a.ts`, `-` (stdin), `--inspect` (REPL). The loader does not transform eval or REPL input. The help text lives in `VPX_HELP` in `crates/vp_global_cli/src/commands/vpx.rs` and the `command_vpx_pnpm*` snapshots.

## Script Detection

Detection runs before package-spec parsing, because script paths can contain `@`. It decides the mode and which token gets the missing-file check; it never rewrites the arguments except to remove `--tsconfig`.

```text
after parsing vpx options, let T = positional[0]

if T starts with '-':                      # Node.js invocation; a leading option never named a package bin
    script = first token after the Node.js options (the token after `--`, or `-` for stdin),
             skipping the value of options that take one (every `--option=...` that
             `node --help` lists on 22, 24, and 26: --import, --require, --env-file, -r, …)
    script is a missing explicit path with a script extension → error
    otherwise → forward to Node.js as-is
elif is_script(T):
    script = T
else:
    package mode (existing lookup chain, unchanged)

is_script(token):
    explicit = token starts with ./ ../ / (Windows: .\ ..\ X:\ \\)
    ext      = token ends with .ts .mts .cts .tsx .js .mjs .cjs .jsx
    ext && cwd/token is a regular file   → script
    ext && (explicit || TS extension)    → error (never a package)
    explicit && cwd/token does not exist → error "Script not found"
    explicit && regular file whose shebang line invokes vpx → script
    otherwise                            → not a script

an error is "Script is a directory" when cwd/token is a directory, else "Script not found"
```

After Node.js options, only a missing explicit path is an error: a bare name may be the value of an option the skip list lacks, and that position never falls back to a package, so a wrong guess only moves the check to Node.js. Both CLIs declare the same lists and run the shared cases in `crates/vp_global_cli/src/commands/vpx_script_cases.json`.

| Invocation                                 | Mode    | Reason                                                     |
| ------------------------------------------ | ------- | ---------------------------------------------------------- |
| `vpx ./scripts/seed.ts`                    | script  | Explicit path with a script extension                      |
| `vpx scripts/seed.ts`                      | script  | The file exists                                            |
| `vpx seed.ts` (no such file)               | error   | TypeScript extension; never a package name                 |
| `vpx ./bin/missing` (no such path)         | error   | Explicit path, even without an extension                   |
| `vpx ./tools.ts` (a directory)             | error   | "Script is a directory"                                    |
| `vpx highlight.js` (no such file)          | package | Bare name with a JS extension and no file                  |
| `vpx ./@scope/tool.ts`                     | script  | Detection runs before `@version` parsing                   |
| `vpx --import ./setup.ts ./main.ts`        | script  | `./setup.ts` is the value of `--import`                    |
| `vpx --eval 'console.log(1)'`              | node    | No script token; forwarded with the loader                 |
| `vpx /abs/bin/tool` (`#!/usr/bin/env vpx`) | script  | Shebang guard; otherwise `vpx` would exec itself in a loop |
| `vpx ./deploy.sh`                          | package | No script extension; runs as a program, without the loader |

## Execution Model

```text
vpx ./scripts/seed.ts --dry-run
 ├─ parse vpx options (--tsconfig, -s, -v, -h; -p/-c select package mode)
 ├─ detect script ── package ──► existing chain: node_modules/.bin → global → PATH → dlx
 ├─ resolve loader: <project vite-plus>/dist, else <global vite-plus>/dist
 ├─ env: prepend every node_modules/.bin (cwd → root); set or clear VP_SCRIPT_TSCONFIG
 └─ core `node` shim dispatch (same runtime selection as `vp node`)
        └─ execve: node --require <dist/script-preload.cjs> --import <file URL of dist/script-register.js>
                        [NODE_OPTIONS] ./scripts/seed.ts --dry-run
```

### Runtime and Process

Script mode hands the argv to the core `node` shim path (`crates/vp_global_cli/src/shim/dispatch.rs`), which `vp node` also uses: project version resolution, system-first mode, `VP_BYPASS`, and inherited runtimes inside `vp run`.

On Unix, `exec_tool` replaces the `vpx` process with `node` through `execve`; no parent remains, so no IPC, signal relay, or exit-code translation is needed. On Windows, the `vpx.exe` trampoline starts `vp.exe`, which spawns `node` and waits, as for every shim; exit codes and Ctrl+C follow the existing `node` shim behavior.

### Loader Entries

Two entries in `vite-plus/dist`, passed together:

1. `--require <dist/script-preload.cjs>` (CommonJS, from `packages/cli/src/script-preload.ts`). It runs before any user `--require` or `--import`, so `--require ./setup.ts` preloads work. It checks the Node.js version and exits with a Vite+ error outside `^22.18.0 || ^24.11.0 || >=26.0.0` (the `engines.node` range), loads the native binding through `packages/cli/binding/index.cjs`, calls `setTsconfigPath()` and `setTransformDeps('typescript')`, turns on source maps, installs the pirates CommonJS hook, and on Node.js 22.22.3+, 24.11.1+, and 26 registers the in-thread ESM hooks with `module.registerHooks()`.
2. `--import <file URL of dist/script-register.js>` (ESM). On older 22.x and 24.x it registers the off-thread hooks (`dist/script-esm-hooks.js`) with `module.register()`, which Node.js 22 cannot call while a `--require` is loading. On every version, an `--import` also routes the entry point through the ESM loader, where the hooks decide each file's module format.

The in-thread hooks start about twice as fast as a hooks thread, and Node.js 25.9+ deprecates `module.register()` (DEP0205). They need nodejs/node#59011 (22.19, 24.5) and nodejs/node#59929 (22.22.3, 24.11.1); tsx switches at the same releases. Upstream oxc-node waits for 26.2 for nodejs/node#62920, which only affects load hooks that return CommonJS source; these hooks do that only when another hook supplied it (see [Known Limitations](#known-limitations)).

### CLI/Loader Contract

The contract is the two flags plus the `VP_SCRIPT_TSCONFIG` environment variable. The global `vp` may be newer or older than the project's `vite-plus`, so it stays stable: new behavior goes into the loader entries, not into new argv. A contract change gets new entry file names, so a `vp` that does not know them falls back to the global loader, as it does for a project too old to ship them. `vpx_script.rs` points readers here for this.

The loader is resolved from the project's `vite-plus` first (`JsExecutor::resolve_local_vite_plus_package_dir`), when its `dist/` has both entries, and from the global install second (`JsExecutor::get_scripts_dir()`), following the global CLI's JS-entry delegation. Teams and CI then run the loader and binding pinned in their lockfile.

### Environment

- Every `node_modules/.bin` from the cwd up is prepended to `PATH` (`prepend_node_modules_bin_to_path`), as for local binaries, so scripts can spawn project tools by name.
- `--tsconfig <path>` is resolved against the cwd and must exist. `vpx` passes it as `VP_SCRIPT_TSCONFIG` and clears an inherited value when the flag is absent. Without the flag, each file uses the nearest tsconfig that includes it, as tsc does; with it, one config applies to every file in the process, for resolution and transform. The preload passes it to `setTsconfigPath()`, which wins over upstream's `TS_NODE_PROJECT` and `OXC_TSCONFIG_PATH`; without the flag those variables apply as for `oxnode`. The [guide](../docs/guide/vpx.md#tsconfig) lists the defaults when no tsconfig applies.
- The loader goes in argv, not `NODE_OPTIONS`: `process.execArgv` carries it into `child_process.fork('./worker.ts')` and `new Worker('./worker.ts')`, while unrelated processes the script spawns (`vp build`, `npm`) do not load it.
- Other loaders are unsupported. A `NODE_OPTIONS=--import tsx` or a user `--import tsx` registers a second set of hooks, and the result depends on which loader claims each file.

## Project-Local `vpx` Bin

`vite-plus` ships `"vpx": "./bin/vpx"` next to `vp` and `vpr` (`packages/cli/src/vpx-bin.ts`, `vpx-script.ts`). Package managers and `vp run` put `node_modules/.bin` first on `PATH`, so this bin runs in `package.json` scripts. Without it, `"seed": "vpx ./scripts/seed.ts"` would work on a developer machine and fail in CI that only runs `npm i -D vite-plus`; tsx has no such gap, and `vp migrate` can only rewrite `tsx` to `vpx` if the result works wherever `tsx` did.

- **Script mode**: the same detection rules, then `spawn(process.execPath, ['--require', preload, '--import', registerUrl, ...nodeArgs], { stdio: 'inherit' })`. The bin ignores `SIGINT`, which reaches the child through the process group, forwards `SIGTERM` and `SIGHUP`, and exits the way the child did (re-raising its signal on Unix, so callers see `128 + signal`).
- **Package mode never hands off to the global `vpx`**, so delegation stays one-directional: the global CLI delegates to the project's `vite-plus`, never the reverse. An earlier design forwarded to the global `vpx` on `PATH`, and two copies of the local bin on `PATH` spawned each other forever.
- The nearest `node_modules/.bin/<name>` from the cwd up runs directly through `cross-spawn`, which runs Windows `.cmd` shims, with every `node_modules/.bin` first on `PATH`, like the global CLI. So does an explicit path to an existing executable file (`./deploy.sh`). It does not use `vp exec`, which needs a `package.json` above the cwd and installs the project's runtime first.
- `pkg@version`, `--package`, `--shell-mode`, and a package directory go to this package's `vp dlx` in-process, by importing `dist/bin.js` with the argv rewritten, as `vpr` does. Only that path loads the CLI bundle and the native binding.
- Global packages and `PATH` are never searched. A binary that is not installed is an error with a hint (add it to `devDependencies`, or run `vp dlx`), never a download. An installed binary without an executable bit is an error with a hint to reinstall.
- In both CLIs, an explicit path to a non-executable file, or to a directory without a `package.json`, is an error (`check_explicit_program`, `checkExplicitProgram`) instead of a `vp dlx` failure.
- `--help` prints the local bin's own help; `--version` prints `vite-plus v<version>`.
- Both CLIs run the shared case table `vpx_script_cases.json`: detection `cases`, `packageSpecs`, and `explicitPrograms`. A unit test checks that their extension and value-taking-option lists are identical.

For a script, the local bin and the global shim resolve the same loader and Node.js; the difference is the local bin's parent process, about 22–26 ms ([Performance](#performance)).

## Native Side

oxc-node is vendored the way Rolldown is:

- `packages/tools/.upstream-versions.json` pins the v0.1.7 commit. The daily `upgrade-deps` workflow bumps it to the latest stable tag, but only to one that contains the pinned commit, so a pin to an unreleased fix stays until a release has it.
- `sync-remote` clones it into the gitignored `oxc-node/` and runs `packages/tools/src/patch-oxc-node.ts`. CI checks it out in the shared clone action and the security workflow and runs the script before any cargo command, because `Cargo.lock` records the rewritten manifest.
- `patch-oxc-node.ts` only points oxc-node's `oxc` dependency at the workspace version, so the binding links one copy of oxc; the source is not patched. The workspace is on oxc 0.154.0, which v0.1.7 asks for, so the rewrite is a no-op today.
- The root `Cargo.toml` adds `oxc-node = { path = "./oxc-node", default-features = false }` and excludes `oxc-node/` from the workspace lints. Without its `napi` feature, oxc-node links as a plain library with no NAPI exports and no global allocator, so nothing collides with the `transform` export that `rolldown_binding` links.
- The binding's `oxc-node` feature compiles `packages/cli/binding/src/oxc_node.rs`; `build.ts` enables `rolldown,oxc-node` for every build, so dev builds compile it too, at no extra crate cost.

`oxc_node.rs` forwards what the loader uses under an `oxcNode` namespace: `createResolve`, `load`, `OxcTransformer` (returning a plain `{ source, sourceMap }`), `setTsconfigPath`, `setTransformDeps`, and `transformsDep`. Its `module_exports` hook calls `init_tracing()` (so `OXC_LOG` works; the subscriber stays off otherwise and never replaces `VP_LOG`'s) and `detect_runtime_syntax()` (so `using` stays native on Node.js 24+), which oxc-node's own module hooks do when its `napi` feature is on. An upstream signature change is a compile error in the forwarders.

Every issue found during this work was fixed upstream in v0.1.5 to v0.1.7, including the `napi` feature, `setTsconfigPath()`, `setTransformDeps("typescript")`, and single-transform CommonJS loading. The vendored copy carries no patch. [oxc-node#835](https://github.com/oxc-project/oxc-node/issues/835) is open (see [Known Limitations](#known-limitations)).

## Loader JavaScript

Files: `packages/cli/src/script-preload.ts`, `script-register.ts`, `script-esm-hooks.ts`, `script-hooks.ts`.

- The entries are adapted from oxc-node's `register.mjs` and `esm.mjs`, using `oxcNode` from the binding. `script-preload.ts` has its own tsdown block: one CommonJS file with `import.meta.url` shimmed and the binding never bundled. `pirates` is a devDependency bundled into `dist`. The off-thread hooks `require()` the binding; importing it would make the hooks thread translate the CommonJS binding for ESM (5–13 ms).
- **Helper mapping.** Lowered code imports runtime helpers. The loader passes oxc-node its own helper module name, `vite-plus:oxc-node`, so it redirects only helpers these hooks emitted, and maps `vite-plus:oxc-node/helpers/<name>` to `@oxc-project/runtime/helpers/<name>`, resolved from `vite-plus` itself: `nextResolve(helper, { parentURL: <vite-plus dist URL> })`. `@oxc-project/runtime` is a dependency of `vite-plus`, so it resolves under strict layouts such as pnpm; its catalog pin follows Rolldown's (0.154.0 today), and neither `sync-remote` nor `upgrade-deps` moves it past that.
- **CommonJS `require` rewrite.** Output without `import`/`export` loads helpers with `require("vite-plus:oxc-node/helpers/…")`, which the off-thread hooks never see. The pirates hook rewrites each to `process.getBuiltinModule("node:module").createRequire(<vite-plus dist URL>)("@oxc-project/runtime/helpers/<name>")`.
- `require()` requests (no `importAttributes` in the context) stay on Node.js resolution and the pirates hook. The in-thread load hook returns a null source for a CommonJS file oxc-node left to the CommonJS loader, so `require()` inside it works on every runtime.
- `setTransformDeps('typescript')` transforms `.ts .mts .cts .tsx` under `node_modules`, which Node.js refuses to strip, and leaves other dependencies as published; `transformsDep()` gives the pirates matcher the same answer. `OXC_TRANSFORM_ALL` has no effect under `vpx`.
- Source maps are on for `node_modules` too (`setSourceMapsSupport(true, { nodeModules: true, generatedCode: true })`), as tsx and upstream do, so stack traces in transformed dependencies point at their source. Node.js then reads the `.map` of every dependency that has one.

## Comparison with tsx

| Capability                     | tsx 4.23                                              | `vpx` (oxc-node 0.1.7)                                                                    | Node.js type stripping |
| ------------------------------ | ----------------------------------------------------- | ----------------------------------------------------------------------------------------- | ---------------------- |
| `.tsx` / `.jsx`                | ✓ (classic runtime by default)                        | ✓ (automatic runtime by default; classic via tsconfig or pragma)                          | ✗                      |
| Enums, namespaces, param props | ✓                                                     | ✓                                                                                         | ✗                      |
| Legacy decorators              | ✓                                                     | ✓, plus `emitDecoratorMetadata`                                                           | ✗                      |
| TC39 decorators                | ✓ (esbuild lowers)                                    | ✗ (clear error)                                                                           | ✗                      |
| tsconfig `paths`               | ✓                                                     | ✓ for `import`; not for `require()`                                                       | ✗                      |
| tsconfig selection             | One per process, from the cwd; `paths` for every file | Nearest config that includes the file (tsc rules); a file no config includes gets neither | n/a                    |
| `.js` → `.ts`, extensionless   | ✓                                                     | ✓                                                                                         | ✗                      |
| `require()` of `.ts`, interop  | ✓ (compiles ESM to CJS if needed)                     | ✓ (`.cts` with ESM syntax is an error)                                                    | Partial                |
| TypeScript in `node_modules`   | ✓                                                     | ✓                                                                                         | ✗                      |
| Class fields, `using`          | Lowered to `target`                                   | Native unless `[[Set]]` fields or legacy decorators; `using` lowered on 22.x              | Native                 |
| Watch                          | `tsx watch` (chokidar)                                | Node.js `--watch`                                                                         | `--watch`              |
| TypeScript REPL / `-e`         | ✓                                                     | ✗                                                                                         | Erasable syntax only   |
| Transform cache                | On disk, about 8 days                                 | None ([Performance](#performance))                                                        | n/a                    |

## Known Limitations

- tsconfig `paths` apply to `import` only; `require()` uses Node.js' CommonJS resolution, as upstream.
- A file that no tsconfig includes resolves without `paths`. With a root config whose `include` is `["src"]`, a script in `scripts/` that imports an alias fails with `ERR_MODULE_NOT_FOUND`, while tsx resolves it. This is the most likely failure when moving from tsx; add the directory to `include` or pass `--tsconfig`. [oxc-node#835](https://github.com/oxc-project/oxc-node/issues/835) asks for a resolution-only fallback to the nearest tsconfig.
- On in-thread releases without nodejs/node#62920 (22.22.3+, 24.11.1 to 24.17, 26.0, 26.1), `require()` inside a CommonJS module whose source another load hook supplied can fail. That needs a second loader, which is unsupported.
- The local bin ignores `SIGINT`, so `kill -INT <bin pid>` does not stop the script; Ctrl+C does, through the process group. Forwarding it would deliver Ctrl+C twice; tsx avoids that with an IPC channel.
- A tsconfig `module` of `node16`, `node18`, or `nodenext` makes `.ts` and `.tsx` files ES modules even without `"type": "module"`, as upstream does; `tsc` and tsx follow `package.json` `"type"`.
- A `.cts` file with `import`/`export` fails with one error on every load path, suggesting `.mts` or `import x = require()` / `export =`. tsc and tsx compile it to CommonJS, which needs an ESM-to-CommonJS transform oxc lacks.
- An extensionless `#!/usr/bin/env vpx` file runs as JavaScript, because oxc-node picks the parser by extension; its syntax decides its module kind, as in Node.js. Shebang scripts keep their `.ts` extension. Options in a shebang need `#!/usr/bin/env -S vpx --tsconfig …`.

## Errors

```bash
$ vpx ./scripts/missing.ts
error: vpx: Script not found: ./scripts/missing.ts

$ vpx -p cowsay ./a.ts
error: vpx: --package cannot be used when running a script

$ vpx --tsconfig x.json eslint .
error: vpx: --tsconfig can only be used when running a script

$ vpx ./a.ts          # engines.node pins 20.18.0
error: Running scripts with vpx requires Node.js ^22.18.0 || ^24.11.0 || >=26.0.0 (current: v20.18.0)
```

Other errors, including transform errors, come from Node.js and oxc-node unchanged; source maps point stack traces at the TypeScript source.

## Design Decisions

1. **Extend `vpx`; leave `vp node` as plain Node.js.** `vpx ./a.ts` reads like `npx tsx ./a.ts`, the shebang is short and already on `PATH`, and `vp node` stays the escape hatch for exact Node.js semantics.
2. **oxc-node** covers the syntax and resolution scripts rely on, shares Oxc with the rest of the toolchain, and is maintained in the same organization. See [Alternatives](#alternatives-considered).
3. **Exec Node.js in place.** The shim is native, so it can `execve` after runtime resolution: `node`'s exact exit codes, signals, and TTY handling, no IPC, one Node.js startup saved.
4. **Vite+-owned loader entries** instead of `--import @oxc-node/core/register`: one place for the version check, helper mapping, and binding loading; the oxc-node version follows the pinned `vite-plus`; the hooks can change without changing the CLI contract; and the entries can later become a public `node --import vite-plus/register`. Why two entries: [Loader Entries](#loader-entries).
5. **Loader in argv, not `NODE_OPTIONS`**, so `fork()` and workers inherit it and unrelated processes do not; tsx does the same.
6. **Explicit paths and bare TypeScript names never download.** `vpx ./sed.ts` must report a missing file, not run remote code under an unexpected name.
7. **Loader from the project first, global second**, as the global CLI already delegates JS entries; teams run the pinned `vite-plus`, at the cost of the version-skew contract.
8. **A project-local `vpx` bin**, so `package.json` scripts work without the global CLI ([Project-Local `vpx` Bin](#project-local-vpx-bin)).
9. **Compile oxc-node into the binding** rather than depend on `@oxc-node/core`. The binding already links the same crates through `rolldown_binding` (oxc-node is about 3,600 lines of glue), so no new platform packages and one oxc version for the whole toolchain. Cost: a new oxc-node tag must compile against the workspace's oxc, and a forwarded signature change needs a matching change in `oxc_node.rs`; either fails the build loudly, and a tag that needs a newer oxc than Rolldown's keeps the daily upgrade PR failing until Rolldown catches up.
10. **Script mode requires the `engines.node` range**, not everything with `module.register()`; Node.js 20 is end of life.
11. **JavaScript files get the loader too**, so `.js` can import `.ts`.
12. **No experimental label.** The vendored copy is pinned to a release and covered by Vite+'s own tests.

## Alternatives Considered

| Alternative                                        | Why not                                                                                                                                    |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Node.js type stripping only                        | No enums, decorators, JSX, `paths`, or `.js` → `.ts`; adds nothing over `vp node`                                                          |
| Bundle tsx                                         | esbuild as a second transformer in an Oxc toolchain; two processes and a signal relay; runs on whatever Node.js starts it                  |
| Pre-bundle with Rolldown                           | Changes `import.meta.url` and `__dirname`, breaks computed dynamic imports, writes temp files                                              |
| Depend on `@oxc-node/core` from npm                | A second native addon with its own 17-target matrix (about 4 MB each); helper mapping still needed; oxc version floats independently       |
| Use the `oxnode` CLI                               | Spawns `node` from `PATH`, ignoring the managed runtime; no signal forwarding; exit code 0 when the child is killed by a signal            |
| Reimplement the hooks in TypeScript on the binding | Reimplements format detection, ESM-in-CommonJS re-parse, JSON modules, and Windows file URLs that oxc-node tests; more NAPI calls per file |

## Testing

- Unit tests in `vpx_script.rs`, `vpx.rs`, and `packages/cli/src/__tests__/vpx-script.spec.ts` run the shared case table, check that the Rust and TypeScript lists match, and cover the local bin's package-mode decision, the loader fallback to the global install, the Node.js range (equal to `engines.node`), the sync-hooks cutoff, and the helper mapping. `patch-oxc-node.spec.ts`, `sync-remote-deps.spec.ts`, and `toolchain.spec.ts` cover the vendoring.
- Snapshot fixtures under `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/` run in both flavors, except one local-only case for a binary that is not installed, and the global shim and the local bin record identical snapshots. `command_vpx_script` pins Node.js 22.18.0 (the `module.register()` path) and covers syntax, resolution and `--tsconfig`, arguments and Node.js options, errors, module formats with `fork()`, `Worker`, and `--test`, TypeScript dependencies, tsconfig-driven transforms and JSX, shebangs and `--watch` (Unix), and package mode. `command_vpx_script_node22` (22.23.2), `_node24` (24.12.0), and `_node26` (26.5.0) cover the `registerHooks()` path; `_old_node` (20.18.0) covers the version error. Windows runs every case except watch mode, shebangs, and the installed-binary case.

## Performance

Startup against tsx 4.23.15 on an Apple M4 Pro, median of 20 runs; `bench/vpx-script.ts` generates the scenarios and the table. The `vpx` loader row is what the global `vpx` execs, without the Rust shim's startup; "no cache" sets `TSX_DISABLE_CACHE=1`. Scenarios: `hello` (one erasable `.ts` file), `esm-app` and `cjs-app` (an entry importing 300 modules with an enum, a class field, and a parameter property, in a `"type": "module"` package and in one without `"type"`), `large-cjs` and `large-esm` (one 1.8 MB `.ts` file in each package kind).

| Scenario    | Command                       | Node.js 22.18.0 | Node.js 24.12.0 | Node.js 26.5.0 |
| ----------- | ----------------------------- | --------------: | --------------: | -------------: |
| `hello`     | `vpx` loader                  |           41 ms |           26 ms |          29 ms |
| `hello`     | `node --import tsx`           |           68 ms |           46 ms |          38 ms |
| `hello`     | `node --import tsx`, no cache |           75 ms |           62 ms |          60 ms |
| `esm-app`   | `vpx` loader                  |           83 ms |           64 ms |          69 ms |
| `esm-app`   | `node --import tsx`           |          116 ms |           85 ms |          81 ms |
| `esm-app`   | `node --import tsx`, no cache |          141 ms |          216 ms |         212 ms |
| `cjs-app`   | `vpx` loader                  |           84 ms |           65 ms |          66 ms |
| `cjs-app`   | `node --import tsx`           |          133 ms |          104 ms |          99 ms |
| `cjs-app`   | `node --import tsx`, no cache |          288 ms |          260 ms |         255 ms |
| `large-cjs` | `vpx` loader                  |          112 ms |           91 ms |          83 ms |
| `large-cjs` | `node --import tsx`           |          122 ms |           89 ms |          75 ms |
| `large-cjs` | `node --import tsx`, no cache |          230 ms |          272 ms |         258 ms |
| `large-esm` | `vpx` loader                  |           95 ms |           76 ms |          76 ms |
| `large-esm` | `node --import tsx`           |          107 ms |           75 ms |          68 ms |
| `large-esm` | `node --import tsx`, no cache |          202 ms |          176 ms |         171 ms |

- `vpx` beats tsx's warm cache everywhere except the megabyte-sized files on 24.12 and 26.5, where tsx skips the transform and is 1–8 ms faster.
- Against tsx without its cache, `vpx` takes 41–75% less time in every scenario.
- On the erasable `hello` file, `vpx` is faster than Node.js type stripping: 41 against 48 ms on 22.18, and 26–29 against 43–48 ms on 24.12 and 26.5.
- The local bin adds 22–26 ms for its second Node.js process; the `tsx` CLI pays the same.
- Compiling oxc-node into the binding adds about 130 KB (0.3%) to the release `libvite_plus_cli.dylib` on macOS arm64, against about 4.2 MB for a separate `@oxc-node/core` platform package; its load cost is within the noise above.

A persistent transform cache would only help megabyte-sized sources.

## Security Considerations

1. **No remote fallback for files**: explicit paths, and bare names with TypeScript extensions, never reach `vp dlx`.
2. **Trusted loader**: the hooks are compiled into the `vite-plus` binding and its `dist`; runtime helpers resolve from `vite-plus`'s own `@oxc-project/runtime`, never from the project's `node_modules`.
3. **Shebang recursion guard** for extensionless `#!/usr/bin/env vpx` files.
4. **Project code provides the loader**, and every `node_modules/.bin` is prepended to `PATH`: the trust model `vp` already uses when it delegates to a project-local `vite-plus`.
5. **No new environment surface**: `VP_SCRIPT_TSCONFIG` is set only with `--tsconfig` (and cleared otherwise) and reaches oxc-node through `setTsconfigPath()`, not through `TS_NODE_PROJECT` or `OXC_TSCONFIG_PATH`, which other tools read.

## Backward Compatibility

Package mode through the global `vpx` is unchanged. What changes:

- `vpx <existing file with a script extension>` runs the file. Previously an executable file with a shebang was exec'd through the `PATH` lookup, and a non-executable file fell through to `vp dlx`, which failed. A bare name matching both a file and a package (`vpx highlight.js` with `./highlight.js` present) runs the file.
- A first token starting with `-` runs Node.js. Previously it reached `vp dlx` as the package name and never ran a package; `vpx --version` printed the package manager's version.
- An explicit path to a non-executable file, or to a directory without a `package.json`, fails with a `vpx` error instead of a package manager error from `vp dlx`.
- `vite-plus` gains a `vpx` bin. A project with a different `vpx` in `node_modules/.bin` (the npm `vpx` package is a 0.0.1 placeholder without a bin) would see a bin conflict warning.
- In `package.json` scripts and `vp run`, that bin shadows the global shim: `vpx <name>` runs only binaries in `node_modules/.bin`. A name found only in global packages or on `PATH` fails with a hint, and a name found nowhere fails instead of being downloaded; `pkg@version`, `--package`, and `--shell-mode` still download. `vpx --version` there prints `vite-plus v<version>`.
- `vite-plus` gains `@oxc-project/runtime` (pure JavaScript) as a dependency; `pirates` is bundled. No new platform packages.

## Open Items

- Windows Ctrl+C through the `vp.exe` spawn-and-wait path is not verified.
- [oxc-node#835](https://github.com/oxc-project/oxc-node/issues/835): `paths` for files outside every tsconfig's `include`.

## Future Enhancements

- **`vp migrate` support**: rewrite `tsx` invocations in `package.json` scripts to `vpx`, only when the project's Node.js satisfies the script-mode range and for the cases under "Coming from tsx" in the guide, and remove the devDependency. `ts-node` (which type-checks by default), `esno`, and `jiti` need their own rules.
- **Public loader entry**: `node --import vite-plus/register ./a.ts`, for tools that must start `node` themselves.
- **TypeScript eval and REPL**, once oxc-node transforms eval input.
- **Watch ergonomics**: `--include`, `--exclude`, and screen clearing beyond Node.js `--watch`.
- **Opt-in transform cache**, keyed like tsx's.
- **Programmatic API**: a `tsImport()` equivalent exported from `vite-plus`.

## References

- tsx source (v4.23.15): https://github.com/privatenumber/tsx/tree/v4.23.15/src; docs: https://tsx.is
- oxc-node source: https://github.com/oxc-project/oxc-node (`packages/core/register.mjs`, `packages/core/esm.mjs`, `src/lib.rs`); docs: https://oxc.rs/docs/guide/usage/oxc-node.html
- Rolldown vendoring in this repository: `packages/tools/src/sync-remote-deps.ts`, `packages/tools/.upstream-versions.json`, `packages/cli/BUNDLING.md`
- Node.js TypeScript support: https://nodejs.org/api/typescript.html; module customization hooks: https://nodejs.org/api/module.html#customization-hooks
