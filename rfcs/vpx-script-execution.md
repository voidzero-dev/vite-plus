# RFC: Run TypeScript Scripts with `vpx`

- Status: Implemented in [#2936](https://github.com/voidzero-dev/vite-plus/pull/2936); CI covers Linux, macOS, and Windows on Node.js 22, 24, and 26
- Related: [`vpx` command](./vpx-command.md), [JavaScript runtime management](./js-runtime.md), [CLI bundling](../packages/cli/BUNDLING.md), [Core binding resolution](./core-binding-resolution.md)
- Upstream: [oxc-node](https://github.com/oxc-project/oxc-node) at `ce22828`, a `main` commit after v0.1.7 with fixes not yet released, vendored unpatched and compiled into the Vite+ native binding, as Rolldown is
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

Type checking (`vp check`); a TypeScript REPL or TypeScript in `-e`/`-p` or `--check`; a programmatic API or public `node --import vite-plus/…` entry (see [Future Enhancements](#future-enhancements)); an on-disk transform cache; `target` downleveling; Bun and Deno.

## Command Syntax

```bash
vpx [VPX_OPTIONS] [NODE_OPTIONS] <script> [args...]
```

- `VPX_OPTIONS`: the existing `vpx` options plus `--tsconfig <path>` and `-v/--version`. `-p/--package` and `-c/--shell-mode` select package mode, so combining them with a script is an error. `-s/--silent` is accepted; script mode prints no Vite+ output either way.
- `NODE_OPTIONS`: tokens starting with `-` between the `vpx` options and the script, forwarded to Node.js verbatim, except two that `vpx` takes out: `--tsconfig` (`vpx --inspect --tsconfig x.json ./a.ts`) and `--watch` (see [Watch Mode](#watch-mode)).
- `<script>` and `[args...]`: passed to Node.js unchanged. Tokens after the script, including `--`, belong to the script.

`vpx` options are parsed first, so `-p`, `-c`, `-h`, and `-v` keep their `vpx` meaning; Node.js has the long forms, and `vp node --version` prints the Node.js version. `vpx --version` prints the same report as `vp --version`.

When the first positional token starts with `-` and no script follows, `vpx` forwards the arguments to Node.js with the loader: `--eval`, `--print`, `--check ./a.ts`, `-` (stdin), `--inspect` (REPL). The loader does not transform eval or REPL input, and Node.js reads a `--check` file without it, as with tsx. The help text lives in `VPX_HELP` in `crates/vp_global_cli/src/commands/vpx.rs` and the `command_vpx_pnpm*` snapshots.

`--watch` among the Node.js options runs oxc-node's watcher instead of Node.js' own watch mode (see [Watch Mode](#watch-mode)). Its options are oxc-node's (`--include`, `--exclude`, `--clear-screen`, `--kill-signal`, `--kill-timeout`, `--rerun-on-enter`, `--poll`), which also accept `tsx watch`'s `--ignore` and `--no-cache` and the Node.js watch mode options `--watch-preserve-output` and `--watch-kill-signal`, and `vpx --watch --help` prints them. They go among the Node.js options, and detection knows which of them take a value (`watchOptionsWithValue` in the shared table), so `vpx --watch --exclude ./gen.ts ./main.ts` runs `main.ts`. `vpx watch` stays package mode, so it still runs the npm package `watch`.

## Script Detection

Detection runs before package-spec parsing, because script paths can contain `@`. It decides the mode and which token gets the missing-file check; it never rewrites the arguments except to remove `--tsconfig` and `--watch`.

```text
after parsing vpx options, let T = positional[0]

if T starts with '-':                      # Node.js invocation; a leading option never named a package bin
    script = first token after the Node.js options (the token after `--`, or `-` for stdin),
             skipping the value of options that take one (every `--option=...` that
             `node --help` lists on 22, 24, and 26: --import, --require, --env-file, -r, …,
             and the watcher's --include, --exclude, --ignore, --kill-signal, --kill-timeout)
    a bare --experimental-config-file takes the next token only when it ends in .json:
             older releases take a separate config path, and 24.19, 26.1, and later read
             the bare flag as node.config.json; a .json file cannot be the script
    after -e, --eval, -p, -pe, --print, --run, or --test, no token is a script: Node.js
             runs that code or package script, or takes test files and globs
    option names match with `_` for `-` (--redirect_warnings), as Node.js accepts
    script is a missing explicit path with a script extension → error
    --watch or --watch-path among the options → the watcher (--watch removed)
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
| `vpx --eval '…' ./missing.ts`              | node    | After `--eval`, `./missing.ts` is an argument              |
| `vpx /abs/bin/tool` (`#!/usr/bin/env vpx`) | script  | Shebang guard; otherwise `vpx` would exec itself in a loop |
| `vpx ./deploy.sh`                          | package | No script extension; runs as a program, without the loader |
| `vpx --watch ./server.ts`                  | watcher | `--watch` runs oxc-node's watcher, not Node.js' watch mode |
| `vpx watch ./server.ts`                    | package | `watch` is the npm package of that name                    |

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

Script mode hands the argv to the core `node` shim path (`crates/vp_global_cli/src/shim/dispatch.rs`), which `vp node` also uses: project version resolution, `VP_BYPASS`, and inherited runtimes inside `vp run`. In system-first mode it first repeats the shell's PATH lookup, as `vp node` does, and runs the system Node.js found before any Vite+ shim.

On Unix, `exec_tool` replaces the `vpx` process with `node` through `execve`; no parent remains, so no IPC, signal relay, or exit-code translation is needed. On Windows, the `vpx.exe` trampoline starts `vp.exe`, which spawns `node` and waits, as for every shim; exit codes and Ctrl+C follow the existing `node` shim behavior.

### Loader Entries

Two entries in `vite-plus/dist`, passed together:

1. `--require <dist/script-preload.cjs>` (CommonJS, from `packages/cli/src/script-preload.ts`). It runs before any user `--require` or `--import`, so `--require ./setup.ts` preloads work. It checks the Node.js version and exits with a Vite+ error outside `^22.18.0 || ^24.11.0 || >=26.0.0` (the `engines.node` range), loads the native binding through `packages/cli/binding/index.cjs`, calls `setTsconfigPath()` and `setTransformDeps('typescript')`, turns on source maps, installs the pirates CommonJS hook, and on Node.js 22.22.3+, 24.11.1+, and 26 registers the in-thread ESM hooks with `module.registerHooks()`.
2. `--import <file URL of dist/script-register.js>` (ESM). On older 22.x and 24.x it registers the off-thread hooks (`dist/script-esm-hooks.js`) with `module.register()`, which Node.js 22 cannot call while a `--require` is loading. On every version, an `--import` also routes the entry point through the ESM loader, where the hooks decide each file's module format.

The in-thread hooks start about twice as fast as a hooks thread, and Node.js 25.9+ deprecates `module.register()` (DEP0205). They need nodejs/node#59011 (22.19, 24.5) and nodejs/node#59929 (22.22.3, 24.11.1); tsx switches at the same releases. Upstream oxc-node waits for 26.2 for nodejs/node#62920, which only affects load hooks that return CommonJS source; these hooks do that only when another hook supplied it (see [Known Limitations](#known-limitations)).

### CLI/Loader Contract

The contract is the two flags plus the `VP_SCRIPT_TSCONFIG` environment variable, and `node <dist/script-watch.js> <watcher args>` for `--watch`. The global `vp` may be newer or older than the project's `vite-plus`, so it stays stable: new behavior goes into the loader entries, not into new argv. A contract change gets new entry file names, so a `vp` that does not know them falls back to the global loader, as it does for a project too old to ship them. `vpx_script.rs` points readers here for this.

The loader is resolved from the project's `vite-plus` first (`JsExecutor::resolve_local_vite_plus_package_dir`), when its `dist/` has both entries (all three for `--watch`), and from the global install second (`JsExecutor::get_scripts_dir()`), following the global CLI's JS-entry delegation. Teams and CI then run the loader and binding pinned in their lockfile. A project `vite-plus` from before the watcher keeps running `vpx <script>` itself and takes the watcher and its loader from the global install.

### Watch Mode

`vpx --watch <script>` never runs Node.js' own watch mode. Detection takes `--watch` out of the Node.js options and marks the invocation, and both CLIs then run `dist/script-watch.js` (from `packages/cli/src/script-watch.ts`) with the remaining arguments. It bundles oxc-node's watcher, `runWatch` from `@oxc-node/cli/watch`, from the vendored `oxc-node/packages/cli/src/watch`, and calls `runWatch(process.argv.slice(2), { name: 'vpx --watch', loaderArgs, ignoreDirs })`: `loaderArgs` are the two loader flags for the `dist/` it sits in, so the watcher and the loader always come from one `vite-plus`, and `ignoreDirs` is that `vite-plus` package, whose loader files and binding show up in the module graph. The name labels the help, the errors, and the `[vpx]` log lines. Only the vpx side changes; oxc-node's watcher runs as published.

- The global `vpx` runs `node <dist>/script-watch.js <args>` through the same `node` shim path as a script, so the watcher gets the Node.js `vp node` selects and, on Unix, the PID of `vpx`. The project-local bin imports the entry in its own process, after putting `node_modules/.bin` first on `PATH`. Either way the watcher process starts the script with `process.execPath`, so there is one process above the script, as Node.js' watch mode had.
- `--tsconfig` works as for a script: the CLIs set or clear `VP_SCRIPT_TSCONFIG` before the watcher starts, the runs inherit it, and the preload passes it to `setTsconfigPath()`, which wins over a `TS_NODE_PROJECT` or `OXC_TSCONFIG_PATH` that an `--env-file` sets. The loader reports the config it read, so the watcher reruns on an edit to it.
- `--watch-path`, which would start Node.js' watch mode, also selects the watcher, which rejects it with a hint to use `--include`. So do `--test`, `--eval`, and the other options that run no file.
- The watcher sets `WATCH_REPORT_DEPENDENCIES` and an IPC channel, so the loader sends its reports, and its own folder watcher sees files created later on every platform and Node.js version. It owns `SIGINT`, `SIGTERM`, `SIGHUP`, stdin (Return reruns), and the IPC channel until it stops, and exits with the script's code.
- Ctrl+C from a terminal reaches the script through the process group, so the watcher does not forward it. When the watcher is not the terminal's foreground job (a pipe, `vp run`, a tool), it forwards a `SIGINT` unless the script acks one within 50 ms on the `OXNODE_SIGNAL_ACK` socket, which the preload's `installSignalAck()` does while the script listens for `SIGINT` itself. A `SIGINT` sent to the watcher's PID alone still reaches the script.

### Environment

- Every `node_modules/.bin` from the cwd up is prepended to `PATH` (`prepend_node_modules_bin_to_path`), as for local binaries, so scripts can spawn project tools by name.
- `--tsconfig <path>` is resolved against the cwd and must exist. `vpx` passes it as `VP_SCRIPT_TSCONFIG` and clears an inherited value when the flag is absent. Without the flag, each file uses the nearest tsconfig that includes it, as tsc does; with it, one config applies to every file in the process, for resolution and transform. The preload passes it to `setTsconfigPath()`, which wins over upstream's `TS_NODE_PROJECT` and `OXC_TSCONFIG_PATH`; without the flag those variables apply as for `oxnode`. A missing file named by them, or a tsconfig that fails to parse, prints a warning instead of being skipped silently. The [guide](../docs/guide/vpx.md#tsconfig) lists the defaults when no tsconfig applies.
- The loader goes in argv, not `NODE_OPTIONS`: `process.execArgv` carries it into `child_process.fork('./worker.ts')` and `new Worker('./worker.ts')`, while unrelated processes the script spawns (`vp build`, `npm`) do not load it.
- Other loaders are unsupported. A `NODE_OPTIONS=--import tsx` or a user `--import tsx` registers a second set of hooks, and the result depends on which loader claims each file.

## Project-Local `vpx` Bin

`vite-plus` ships `"vpx": "./bin/vpx"` next to `vp` and `vpr` (`packages/cli/src/vpx-bin.ts`, `vpx-script.ts`). Package managers and `vp run` put `node_modules/.bin` first on `PATH`, so this bin runs in `package.json` scripts. Without it, `"seed": "vpx ./scripts/seed.ts"` would work on a developer machine and fail in CI that only runs `npm i -D vite-plus`; tsx has no such gap, and `vp migrate` can only rewrite `tsx` to `vpx` if the result works wherever `tsx` did.

- **Script mode**: the same detection rules, then Node.js with `['--require', preload, '--import', registerUrl, ...nodeArgs]`. On Unix, the bin execs it in place with `process.execve()`, as the global `vpx` does, so the PID its caller holds, every signal sent to it, and the exit status belong to the script. The new process starts as a spawned child would: the bin restores the default `SIGPIPE` and `SIGXFSZ` dispositions, which Node.js ignores, and blocking stdio, which `process.execve()` leaves non-blocking once Node.js has opened a pipe (nodejs/node#66824).
- On Windows, and when the bin itself has an IPC channel, as when a tool `fork()`s it, the bin spawns the script instead, since exec would close the channel. It ignores `SIGINT`, which reaches the child through the process group, forwards `SIGTERM` and `SIGHUP`, and exits the way the child did (re-raising its signal on Unix, so callers see `128 + signal`). With an IPC channel, it opens one to the script and relays messages both ways, as tsx does, and exits with the script's code once the script has exited and the channel has closed, whichever side closed it. The global `vpx` passes the channel on through exec. On Windows, only a `fork()` of the bin file carries a channel; a `.cmd` shim cannot.
- **Package mode never hands off to the global `vpx`**, so delegation stays one-directional: the global CLI delegates to the project's `vite-plus`, never the reverse. An earlier design forwarded to the global `vpx` on `PATH`, and two copies of the local bin on `PATH` spawned each other forever.
- The nearest `node_modules/.bin/<name>` from the cwd up runs directly, with every `node_modules/.bin` first on `PATH`, like the global CLI. On Unix, the bin execs it in place: directly when it is a native binary or a script whose `#!` interpreter is one, and otherwise through `/bin/sh -c 'exec "$0" "$@"'`, because a failed `process.execve()` aborts most supported Node.js versions and sh reports a file it cannot run as a terminal does. On Windows, `cross-spawn` runs it, including `.cmd` shims. So does an explicit path to an existing executable file (`./deploy.sh`). It does not use `vp exec`, which needs a `package.json` above the cwd and installs the project's runtime first.
- `pkg@version`, `--package`, `--shell-mode`, and a package directory go to this package's `vp dlx` in-process, by importing `dist/bin.js` with the argv rewritten, as `vpr` does. Only that path loads the CLI bundle and the native binding.
- Global packages and `PATH` are never searched. A binary that is not installed is an error with a hint (add it to `devDependencies`, or run `vp dlx`), never a download. An installed binary without an executable bit is an error with a hint to reinstall.
- In both CLIs, an explicit path to a non-executable file, or to a directory without a `package.json`, is an error (`check_explicit_program`, `checkExplicitProgram`) instead of a `vp dlx` failure.
- `--help` prints the local bin's own help; `--version` prints `vite-plus v<version>`.
- Both CLIs run the shared case table `vpx_script_cases.json`: detection `cases`, `packageSpecs`, and `explicitPrograms`. A unit test checks that their extension, value-taking-option, and no-entry-option lists are identical.

For a script, the local bin and the global shim resolve the same loader and Node.js; the difference is the local bin's own Node.js startup before it execs the script, about 23–30 ms against the global shim's 5–10 ms ([Performance](#performance)).

## Migration from tsx

`vp migrate` rewrites tsx commands to `vpx` as an optional migration, like Prettier and tsup: always on a fresh project, and with `--full` on one already using Vite+ (`packages/cli/src/migration/migrator/tsx.ts`; behavior in [Migration Rules](../docs/guide/migrate-rules.md#tsx-to-vpx)).

- It reads commands with the same literal-command parser as the other script rewrites and finds the script the way `vpx` does, with the shared value-taking option lists. Commands it cannot read, and forms `vpx` cannot run (eval, `--check`, the REPL, `--test` without files or in watch mode, an entry without an extension), keep tsx with a warning.
- It has no Node.js check of its own: `vpx` needs the `engines.node` range of `vite-plus` (`SUPPORTED_NODE_RANGE` in `script-hooks.ts`, which a unit test keeps equal), and migration already upgrades incompatible runtime pins to that range for Vite+ itself.
- The `tsx` devDependency goes only when no scripts, imports (including build output and `package.json#imports` aliases), shebangs, tool settings such as `--import=tsx`, CI workflows, git hooks, or shell scripts use tsx anymore, including in nested packages that can resolve it. A package that lists tsx in `dependencies`, `optionalDependencies`, or `peerDependencies` keeps it, since `vpx` comes from a devDependency.
- It reads the scripts a command runs, and its local preloads, and keeps tsx when one does not exist yet, as when an earlier command generates it, and when `vpx` would fail on one: a `require()` of `./a.js` that only tsx resolves to `a.ts`, a `.cts` file with `import` or `export`, and a file that mixes `import` with `__dirname`, `__filename`, `require()`, or `module.exports` in a package without `"type": "module"`. tsx compiles the last two to CommonJS; `vpx` rejects the first of them and runs the second as an ES module. These checks are exact and local. The migration skips globs and scripts after a shell state change, and does not look for differences that need module-graph or tsconfig analysis (imports of the named files, standard decorators, tsconfig `paths` in `require()`, a tsconfig `module` of `node16` or `nodenext`); `vpx` reports those when the script runs.

## Native Side

oxc-node is vendored the way Rolldown is:

- `packages/tools/.upstream-versions.json` pins `ce22828`, a `main` commit after v0.1.7 that has fixes not yet released. The daily `upgrade-deps` workflow bumps it to the latest stable tag, but only to one that contains the pinned commit, so the pin stays until a release has those fixes.
- `sync-remote` clones it into the gitignored `oxc-node/` and runs `packages/tools/src/patch-oxc-node.ts`. CI checks it out in the shared clone action and the security workflow and runs the script before any cargo command, because `Cargo.lock` records the rewritten manifest.
- `patch-oxc-node.ts` only points oxc-node's `oxc` dependency at the workspace version, so the binding links one copy of oxc; the source is not patched. oxc-node's `main` asks for oxc 0.155.0 and builds unchanged with the workspace's 0.154.0, which Rolldown pins.
- The root `Cargo.toml` adds `oxc-node = { path = "./oxc-node", default-features = false, features = ["yarn_pnp"] }` and excludes `oxc-node/` from the workspace lints. `yarn_pnp` matches the `oxc_resolver` feature the workspace enables, which changes oxc-node's file system signature. Without its `napi` feature, oxc-node links as a plain library with no NAPI exports and no global allocator, so nothing collides with the `transform` export that `rolldown_binding` links.
- The binding's `oxc-node` feature compiles `packages/cli/binding/src/oxc_node.rs`; `build.ts` enables `rolldown,oxc-node` for every build, so dev builds compile it too, at no extra crate cost.

`oxc_node.rs` forwards what the loader uses under an `oxcNode` namespace: `createResolve`, `load`, `OxcTransformer` (returning a plain `{ source, sourceMap }`), `setTsconfigPath`, `setTransformDeps`, `transformsDep`, and the watch reporting, `enableWatchReport` and `takeWatchDependencies`. Its `module_exports` hook calls `init_tracing()` (so `OXC_LOG` works; the subscriber stays off otherwise and never replaces `VP_LOG`'s) and `detect_runtime_syntax()` (so `using` stays native on Node.js 24+), which oxc-node's own module hooks do when its `napi` feature is on. An upstream signature change is a compile error in the forwarders.

Issues found during this work were fixed upstream: in v0.1.5 to v0.1.7, including the `napi` feature, `setTsconfigPath()`, `setTransformDeps("typescript")`, and single-transform CommonJS loading, and on `main` since, all the ones found in end-to-end testing against tsx ([oxc-node#835](https://github.com/oxc-project/oxc-node/issues/835), [#837](https://github.com/oxc-project/oxc-node/issues/837), [#839](https://github.com/oxc-project/oxc-node/issues/839) to [#846](https://github.com/oxc-project/oxc-node/issues/846), and [#856](https://github.com/oxc-project/oxc-node/issues/856)), and the one from the final round, [#862](https://github.com/oxc-project/oxc-node/issues/862) (a parent URL ending in `/`, the working directory Node.js resolves the entry point and `--import` against, was resolved as a file). The vendored copy carries no patch. Two oxc transformer issues found in that testing are still open; see [Known Limitations](#known-limitations).

## Loader JavaScript

Files: `packages/cli/src/script-preload.ts`, `script-register.ts`, `script-esm-hooks.ts`, `script-hooks.ts`.

- The entries are adapted from oxc-node's `register.mjs` and `esm.mjs`, using `oxcNode` from the binding. `script-preload.ts` has its own tsdown block: one CommonJS file with `import.meta.url` shimmed and the binding never bundled. `pirates` is a devDependency bundled into `dist`. The off-thread hooks `require()` the binding; importing it would make the hooks thread translate the CommonJS binding for ESM (5–13 ms).
- **Helper mapping.** Lowered code imports runtime helpers. The loader passes oxc-node its own helper module name, `vite-plus:oxc-node`, so it redirects only helpers these hooks emitted, and maps `vite-plus:oxc-node/helpers/<name>` to `@oxc-project/runtime/helpers/<name>`, resolved from `vite-plus` itself: `nextResolve(helper, { parentURL: <vite-plus dist URL> })`. `@oxc-project/runtime` is a dependency of `vite-plus`, so it resolves under strict layouts such as pnpm; `upgrade-deps` keeps its catalog pin at the latest oxc release, never older than the oxc version the binding transforms with, so every helper the transformer emits exists.
- **CommonJS `require` of helpers.** Output without `import`/`export` loads helpers with `require("vite-plus:oxc-node/helpers/…")`, which never reaches the off-thread hooks, while the `registerHooks()` resolve hook passes every `require()` on unchanged to Node.js' CommonJS resolution, which resolves from the requiring file and ignores a `parentURL`. The preload wraps `Module._resolveFilename`, which both go through, to resolve those specifiers from `vite-plus`, as upstream's `register.mjs` does, so a project without `@oxc-project/runtime` (pnpm, or a script outside any project) still finds them.
- `require()` requests (no `importAttributes` in the context) stay on Node.js resolution and the pirates hook. The in-thread load hook returns a null source for a CommonJS file oxc-node left to the CommonJS loader, so `require()` inside it works on every runtime.
- The CommonJS hook gives oxc-node's error for `require()` of an ES module with top-level await Node.js's `ERR_REQUIRE_ASYNC_MODULE` code, which the binding cannot set, as upstream's `register.mjs` does.
- `setTransformDeps('typescript')` transforms `.ts .mts .cts .tsx` under `node_modules`, which Node.js refuses to strip, and leaves other dependencies as published; `transformsDep()` gives the pirates matcher the same answer. `OXC_TRANSFORM_ALL` has no effect under `vpx`.
- Source maps are on for `node_modules` too (`setSourceMapsSupport(true, { nodeModules: true, generatedCode: true })`), as tsx and upstream do, so stack traces in transformed dependencies point at their source. Node.js then reads the `.map` of every dependency that has one.
- **Watch reports.** Under a watcher (`WATCH_REPORT_DEPENDENCIES` and an IPC channel, which the watcher sets up), the preload calls `enableWatchReport()` before the first transform or resolve, and each hook and the pirates transform send what the binding recorded, the tsconfig files read and the paths a failed resolve looked for, as `{ "watch:import": [url] }`, the message oxc-node's watcher and Node.js' read. The sender and the relay are oxc-node's `@oxc-node/core/watch-report`, bundled from the vendored source. The preload and the `--import` entry are separate bundles, so they share the thread's sink through a global symbol. The off-thread hooks have no IPC channel; `script-register.ts` passes them a port, as upstream's `register.mjs` does, and relays what they post. The preload also installs `@oxc-node/core/signal-ack`, the app side of the watcher's Ctrl+C handshake, a no-op without `OXNODE_SIGNAL_ACK`.

## Comparison with tsx

| Capability                     | tsx 4.23                                              | `vpx`                                                                                                                              | Node.js type stripping |
| ------------------------------ | ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | ---------------------- |
| `.tsx` / `.jsx`                | ✓ (classic runtime by default)                        | ✓ (automatic runtime by default; classic via tsconfig or pragma)                                                                   | ✗                      |
| Enums, namespaces, param props | ✓                                                     | ✓                                                                                                                                  | ✗                      |
| Legacy decorators              | ✓                                                     | ✓, plus `emitDecoratorMetadata`                                                                                                    | ✗                      |
| TC39 decorators, `accessor`    | ✓ (esbuild lowers)                                    | ✗ (decorators: clear error)                                                                                                        | ✗                      |
| `SIGINT` / `SIGTERM`           | Exit code 130 / 143, `exit` handlers run              | The signal ends the process, as in Node.js                                                                                         | Same as `vpx`          |
| tsconfig `paths`               | ✓                                                     | ✓ for `import`; not for `require()`                                                                                                | ✗                      |
| tsconfig selection             | One per process, from the cwd; `paths` for every file | Nearest config that includes the file (tsc rules); a file no config includes gets default options and the nearest config's `paths` | n/a                    |
| `.js` → `.ts`, extensionless   | ✓                                                     | ✓ for `import`; not for `require()`                                                                                                | ✗                      |
| `require()` of `.ts`, interop  | ✓ (compiles ESM to CJS if needed)                     | ✓ (`.cts` with ESM syntax is an error)                                                                                             | Partial                |
| TypeScript in `node_modules`   | ✓                                                     | ✓                                                                                                                                  | ✗                      |
| Class fields, `using`          | Lowered to `target`                                   | Native unless `[[Set]]` fields or legacy decorators; `using` lowered on 22.x                                                       | Native                 |
| Watch                          | `tsx watch` (chokidar)                                | `vpx --watch`: oxc-node's watcher, also tsconfig edits and created files                                                           | `--watch`              |
| TypeScript REPL / `-e`         | ✓                                                     | ✗                                                                                                                                  | Erasable syntax only   |
| Transform cache                | On disk, about 8 days                                 | None ([Performance](#performance))                                                                                                 | n/a                    |

## Known Limitations

- `require()` uses Node.js' CommonJS resolution, as upstream: it applies no tsconfig `paths` and does not map `.js` to `.ts`. tsx does both.
- A `.ts` or `.js` file with `import`, `export`, or `import.meta` in a package without `"type"` runs as an ES module, as in Node.js, so `__filename`, `__dirname`, and `require` are undefined. tsx compiles it to CommonJS.
- `SIGINT` and `SIGTERM` end the script as they end Node.js. tsx turns them into exit codes 130 and 143 and runs `exit` handlers first.
- On the off-thread releases (22.18 to 22.22.2, 24.11.0), a `--require` preload also runs in the `module.register()` hooks thread, so it runs twice. tsx has the same behavior.
- On the off-thread releases, starting the hooks thread makes Node.js open stdout and stderr, which sets a pipe non-blocking, so `fs.writeSync(1, …)` to a slow reader can fail with `EAGAIN`. `process.stdout.write()` is not affected. tsx has the same behavior.
- A `--require` or `--import` in `NODE_OPTIONS` also runs in the local bin's own Node.js before it execs the script, so it runs twice, as under the `tsx` CLI. The global `vpx` runs it once.
- A bare `--experimental-config-file` takes the next argument in detection only when it ends in `.json`, the same on every Node.js release: older releases take a separate config path, and 24.19, 26.1, and later read the bare flag as `node.config.json`. On an older release, a separate config path that does not end in `.json` is taken for the script, so `--watch` and `--tsconfig` after it are not vpx's; `--experimental-config-file=<path>` works on every release.
- `vpx --watch` puts its Ctrl+C ack socket in `TMPDIR`. When that path would pass 100 bytes (the `sun_path` limit), the watcher runs without the ack, so a Ctrl+C while it is not the terminal's foreground job reaches a script that handles `SIGINT` twice, as it would without the loader's ack.
- On in-thread releases without nodejs/node#62920 (22.22.3+, 24.11.1 to 24.17, 26.0, 26.1), `require()` inside a CommonJS module whose source another load hook supplied can fail. That needs a second loader, which is unsupported.
- When a tool `fork()`s the local bin, the bin spawns the script and ignores `SIGINT`, so `kill -INT <bin pid>` does not stop the script; Ctrl+C does, through the process group. Forwarding it would deliver Ctrl+C twice; tsx avoids that with its own IPC channel.
- On Windows, the local bin runs a `.cmd` binary from `node_modules/.bin` through `cmd.exe`, so Ctrl+C there can end with the "Terminate batch job (Y/N)?" prompt. The global `vpx` avoids it by running the binary's `.ps1` sibling, when one exists and stdin is a terminal.
- A tsconfig `module` of `node16`, `node18`, or `nodenext` makes `.ts` and `.tsx` files ES modules even without `"type": "module"`, as upstream does; `tsc` and tsx follow `package.json` `"type"`.
- A `.cts` file with `import`/`export` fails with one error on every load path, suggesting `.mts` or `import x = require()` / `export =`. tsc and tsx compile it to CommonJS, which needs an ESM-to-CommonJS transform oxc lacks.
- An extensionless `#!/usr/bin/env vpx` file runs as JavaScript, because oxc-node picks the parser by extension; its syntax decides its module kind, as in Node.js. Shebang scripts keep their `.ts` extension. Options in a shebang need `#!/usr/bin/env -S vpx --tsconfig …`.

These oxc transformer issues found in end-to-end testing against tsx are still open:

| Issue                                                        | Effect under `vpx`                                                                         |
| ------------------------------------------------------------ | ------------------------------------------------------------------------------------------ |
| [oxc#27741](https://github.com/oxc-project/oxc/issues/27741) | `accessor` fields are not lowered, so Node.js rejects them                                 |
| [oxc#27742](https://github.com/oxc-project/oxc/issues/27742) | Legacy decorators on static members run before those on later instance members, unlike tsc |

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
3. **Exec Node.js in place.** The global shim is native, so it can `execve` after runtime resolution: `node`'s exact exit codes, signals, and TTY handling, no IPC, one Node.js startup saved. The local bin execs too on Unix (`process.execve()`), so a caller holding its PID holds the script's; it spawns only on Windows and when it has an IPC channel to relay.
4. **Vite+-owned loader entries** instead of `--import @oxc-node/core/register`: one place for the version check, helper mapping, and binding loading; the oxc-node version follows the pinned `vite-plus`; the hooks can change without changing the CLI contract; and the entries can later become a public `node --import vite-plus/register`. Why two entries: [Loader Entries](#loader-entries).
5. **Loader in argv, not `NODE_OPTIONS`**, so `fork()` and workers inherit it and unrelated processes do not; tsx does the same.
6. **Explicit paths and bare TypeScript names never download.** `vpx ./sed.ts` must report a missing file, not run remote code under an unexpected name.
7. **Loader from the project first, global second**, as the global CLI already delegates JS entries; teams run the pinned `vite-plus`, at the cost of the version-skew contract.
8. **A project-local `vpx` bin**, so `package.json` scripts work without the global CLI ([Project-Local `vpx` Bin](#project-local-vpx-bin)).
9. **Compile oxc-node into the binding** rather than depend on `@oxc-node/core`. The binding already links the same crates through `rolldown_binding` (oxc-node is about 3,600 lines of glue), so no new platform packages and one oxc version for the whole toolchain. Cost: a new oxc-node tag must compile against the workspace's oxc, and a forwarded signature change needs a matching change in `oxc_node.rs`; either fails the build loudly, and a tag that needs a newer oxc than Rolldown's keeps the daily upgrade PR failing until Rolldown catches up.
10. **Script mode requires the `engines.node` range**, not everything with `module.register()`; Node.js 20 is end of life.
11. **JavaScript files get the loader too**, so `.js` can import `.ts`.
12. **No experimental label.** The vendored copy is pinned to a known commit and covered by Vite+'s own tests.

## Alternatives Considered

| Alternative                                        | Why not                                                                                                                                    |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Node.js type stripping only                        | No enums, decorators, JSX, `paths`, or `.js` → `.ts`; adds nothing over `vp node`                                                          |
| Bundle tsx                                         | esbuild as a second transformer in an Oxc toolchain; two processes and a signal relay; runs on whatever Node.js starts it                  |
| Pre-bundle with Rolldown                           | Changes `import.meta.url` and `__dirname`, breaks computed dynamic imports, writes temp files                                              |
| Depend on `@oxc-node/core` from npm                | A second native addon with its own 17-target matrix (about 4 MB each); helper mapping still needed; oxc version floats independently       |
| Use the `oxnode` CLI                               | Spawns `node` from `PATH`, ignoring the managed runtime, and stays as a parent process that relays signals                                 |
| Reimplement the hooks in TypeScript on the binding | Reimplements format detection, ESM-in-CommonJS re-parse, JSON modules, and Windows file URLs that oxc-node tests; more NAPI calls per file |

## Testing

- Unit tests in `vpx_script.rs`, `vpx.rs`, and `packages/cli/src/__tests__/vpx-script.spec.ts` run the shared case table, check that the Rust and TypeScript lists match, and cover the local bin's package-mode decision and which binaries it execs without `/bin/sh` (`canExecDirectly`), the loader fallback to the global install and its real path, the Node.js range (equal to `engines.node`), the sync-hooks cutoff, and the helper mapping. `patch-oxc-node.spec.ts`, `sync-remote-deps.spec.ts`, and `toolchain.spec.ts` cover the vendoring.
- Snapshot fixtures under `crates/vp_cli_snapshots/tests/cli_snapshots/fixtures/` run in both flavors, except one local-only case for a binary that is not installed, and the global shim and the local bin record identical snapshots. `command_vpx_script` pins Node.js 22.18.0 (the `module.register()` path) and covers syntax, resolution and `--tsconfig`, arguments and Node.js options, errors, module formats with `fork()`, `Worker`, and `--test`, TypeScript dependencies, tsconfig-driven transforms and JSX, shebangs, `--watch` (an import edit, a tsconfig edit, a created file, both `--tsconfig` positions with an env file naming another config, the tsconfig edit on 24.12.0 for the `registerHooks()` path, its help and errors, and Ctrl+C to the process group, acked once), a bare `--experimental-config-file` on 26.12.0 and a separate `.json` config path on 22.18.0 (a script argument named `--watch`, then `--tsconfig` and the watcher after the flag or its path), IPC, and signals sent to the PID of `vpx` (Unix), file names with `#` and `%`, and package mode. `command_vpx_script_node22` (22.23.2), `_node24` (24.12.0), and `_node26` (26.5.0) cover the `registerHooks()` path, including CommonJS output that requires runtime helpers the project cannot resolve; `_old_node` (20.18.0) covers the version error. `shim_system_first_fallback` checks that script mode follows `vp node` in system-first mode. Windows runs every case except watch mode, shebangs, IPC, and signals, which a `.cmd` shim cannot carry, and runs its own installed-binary case with a `.cmd` binary.

## Performance

Startup against tsx 4.23.15 on an Apple M4 Pro, median of 20 runs; `bench/vpx-script.ts --vpx <path>` generates the scenarios and the numbers. Scenarios: `hello` (one erasable `.ts` file), `esm-app` and `cjs-app` (an entry importing 300 modules with an enum, a class field, and a parameter property, in a `"type": "module"` package and in one without `"type"`), `large-cjs` and `large-esm` (one 1.8 MB `.ts` file in each package kind). "No cache" sets `TSX_DISABLE_CACHE=1`.

Each cell is `vpx` / tsx with a warm cache / tsx without a cache, in ms. The global `vpx` runs as one process, so it is compared with `node --import tsx`:

| Scenario    |   Node.js 22.18 |  Node.js 24.12 |   Node.js 26.5 |
| ----------- | --------------: | -------------: | -------------: |
| `hello`     |    51 / 68 / 75 |   34 / 46 / 63 |   37 / 38 / 61 |
| `esm-app`   |  94 / 124 / 141 |  75 / 94 / 217 |  77 / 92 / 216 |
| `cjs-app`   |  94 / 143 / 289 | 75 / 116 / 261 | 77 / 113 / 257 |
| `large-cjs` | 121 / 122 / 229 | 100 / 90 / 273 |  93 / 76 / 258 |
| `large-esm` | 103 / 107 / 200 |  84 / 75 / 177 |  86 / 69 / 173 |

The project-local `vpx`, which `package.json` scripts run, starts its own Node.js before it execs the script, as the `tsx` CLI starts a parent process, so it is compared with the `tsx` CLI:

| Scenario    |   Node.js 22.18 |   Node.js 24.12 |    Node.js 26.5 |
| ----------- | --------------: | --------------: | --------------: |
| `hello`     |  66 / 103 / 110 |    52 / 83 / 96 |    57 / 72 / 94 |
| `esm-app`   | 113 / 165 / 177 |  93 / 134 / 250 |  98 / 134 / 249 |
| `cjs-app`   | 112 / 184 / 327 |  93 / 155 / 296 |  99 / 155 / 291 |
| `large-cjs` | 139 / 155 / 265 | 118 / 122 / 305 | 113 / 111 / 294 |
| `large-esm` | 122 / 139 / 232 | 103 / 108 / 210 | 107 / 104 / 206 |

- The global shim's startup, Node.js resolution, and loader lookup take 5–10 ms, usually 8. The local bin's own Node.js startup adds 23–30 ms before it execs the script, against 32–42 ms for the `tsx` CLI's parent process. Exec in place keeps startup within 2 ms of spawning the script; it saves the second process, not time.
- On `hello`, `esm-app`, and `cjs-app`, `vpx` beats tsx's warm cache on every version: by 4–36% through the global shim and 22–40% through the local bin.
- Without tsx's cache, `vpx` takes 33–71% less time everywhere.
- tsx's cache wins only on the 1.8 MB files through the global shim on 24.12 and 26.5, by 9–17 ms, because `vpx` transforms them on every run; a persistent transform cache would only help such files. Against the `tsx` CLI, the local bin is faster or level.
- On `hello`, the loader alone beats plain `node` type stripping on every version (26–41 against 44–48 ms). With the shim, the global `vpx` is 3 ms slower on 22.18 and 10–11 ms faster on 24.12 and 26.5.
- Compiling oxc-node into the binding adds about 150 KB (0.3%) to the release `libvite_plus_cli.dylib` on macOS arm64, against about 4.2 MB for a separate `@oxc-node/core` platform package; its load cost is within the noise above.

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

- Windows Ctrl+C through the `vp.exe` spawn-and-wait path: `vp.exe` handles Ctrl+C and Ctrl+Break while it waits for the child (`vp_command::ConsoleCtrlGuard` in `spawn_tool`, `run_command`, and `execute_with_terminal_guard`, as the `vpx.exe` trampoline and `TerminalStateGuard` on Unix do), so the global `vpx` and `vp run` exit after the script's own cleanup. `vp_run_vpx_script_ctrl_c_cleanup` checks that the global vpx and `vp run` exit after a 300 ms `SIGINT` cleanup, with the script's code. It runs the script as a `vp run` task because the snapshot runner starts steps with Ctrl+C ignored, which only vite-task clears.

## Future Enhancements

- **`vp migrate` rules for other runners**: `ts-node` (which type-checks by default), `esno`, and `jiti` need their own rules.
- **Public loader entry**: `node --import vite-plus/register ./a.ts`, for tools that must start `node` themselves.
- **TypeScript eval and REPL**, once oxc-node transforms eval input.
- **Opt-in transform cache**, keyed like tsx's.
- **Programmatic API**: a `tsImport()` equivalent exported from `vite-plus`.

## References

- tsx source (v4.23.15): https://github.com/privatenumber/tsx/tree/v4.23.15/src; docs: https://tsx.is
- oxc-node source: https://github.com/oxc-project/oxc-node (`packages/core/register.mjs`, `packages/core/esm.mjs`, `src/lib.rs`); docs: https://oxc.rs/docs/guide/usage/oxc-node.html
- Rolldown vendoring in this repository: `packages/tools/src/sync-remote-deps.ts`, `packages/tools/.upstream-versions.json`, `packages/cli/BUNDLING.md`
- Node.js TypeScript support: https://nodejs.org/api/typescript.html; module customization hooks: https://nodejs.org/api/module.html#customization-hooks
