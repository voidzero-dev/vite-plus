# RFC: Bundle Cache for `vpx` Scripts

- Status: Experimental, opt-in with `VPX_BUNDLE_CACHE=1`
- Related: [Run TypeScript scripts with `vpx`](./vpx-script-execution.md), [closed transform-cache prototype (#3343)](https://github.com/voidzero-dev/vite-plus/pull/3343)
- Code: `packages/cli/src/script-bundle.ts`, `script-bundle-plan.ts`, `script-bundle-hooks.ts`, `script-bundle-build.ts`, and the switch in `script-preload.ts`

## Summary

With `VPX_BUNDLE_CACHE=1`, `vpx <script>` bundles the ES modules of a script with Rolldown after the first run. Later runs load the bundle when no input changed. On the [Knip workload](#results) of #3343, a complete run through the global `vpx` takes 153.7 ms instead of 307.4 ms on Node.js 24.22.2 (2.00x).

```
first run    node + loader ──► script, one module at a time ──► exit: write trace, start builder
                                                                 builder ──► Rolldown ──► cache
later runs   node + loader ──► check inputs ──► entry.mjs (+ chunks) ──► script
```

## Why the transform cache could not reach 2x

#3343 cached the Oxc transform of each module. It saved little, because the transform is not where the time goes. A profile of the Knip run with the current loader (Node.js 24.22.2, Apple M4 Pro) splits the 248 ms before the first statement of `cli.ts` like this:

| Startup cost (about 670 modules) |        Time |
| -------------------------------- | ----------: |
| V8 compile                       |       53 ms |
| Native resolve hook              |       41 ms |
| Node.js module loader            |       34 ms |
| Oxc load and transform           |       25 ms |
| File system calls                |       25 ms |
| Top-level code of all modules    | about 15 ms |

- The transform takes about 28 µs for a 2.5 KB file. A cache lookup (hash, read, verify) costs about the same.
- A per-module cache does not change the compile, resolve, loader, and file system costs.
- Plain `node` with its own type stripping and its compile cache is the lower limit for any loader that keeps one file per module. It runs the Knip scan in 234.7 ms, 1.34x faster than the current `vpx`.

Thus only a change to the shape of the module graph can reach 2x. The bundle removes the per-module work for every module it contains.

## How the cache works

### Files

The cache is in the nearest `node_modules` directory above the script. It has one directory for each entry and key (see [the key](#the-key-which-cache-a-run-uses)), and one shared compile cache:

```
<project>/node_modules/.vite-plus/vpx-bundle/
├── cli.ts-1q17scsmimr/       one entry and key: the file name and a hash of the key
│   ├── trace.json            what the last recorded run resolved and loaded
│   ├── manifest.json         the result of the last build: inputs, output, or why there is none
│   ├── excluded.json         modules that later builds must leave out (from the duplicate check)
│   ├── build.lock/           exists while a builder runs
│   └── out-<id>/             the bundle: entry.mjs, chunk-*.mjs, and their source maps
└── v8/                       Node.js' compile cache for the chunks
```

`manifest.json` holds the full key, the name of the output directory, its chunk files, the URLs of the bundled modules, and the list of inputs. An input is a path and its stamp. A manifest without an output has a `disabled` reason instead.

### The key: which cache a run uses

The key selects the entry's directory. It holds everything outside the files that changes how the script loads:

- The Node.js version, the platform, and the architecture.
- The entry's real path and the working directory, where tsconfig discovery starts.
- `execArgv`, for example `--conditions`, and `NODE_OPTIONS`.
- The variables that select a tsconfig (`VP_SCRIPT_TSCONFIG`, `TSX_TSCONFIG_PATH`, `TS_NODE_PROJECT`, `OXC_TSCONFIG_PATH`).

The directory name has a hash of the key, and the manifest stores the full key. The script's own arguments are not in the key: `vpx ./cli.ts lint` and `vpx ./cli.ts build` use the same bundle.

### Each run: hit, off, or miss

At startup, before any module loads, the preload decides from `manifest.json`:

```
VPX_BUNDLE_CACHE=1, and no unsupported option? ── no ──► normal run
        │ yes
manifest exists, same version and key? ─────────── no ──► miss
        │ yes
every input has its stamp? ─────────────────────── no ──► miss ("<first changed file> changed")
        │ yes
manifest has a `disabled` reason? ──────────────── yes ─► off: normal run, nothing recorded
        │ no
every chunk exists? ────────────────────────────── no ──► miss
        │ yes
        ▼
       hit: run the bundle
```

- **Hit.** The run loads `entry.mjs` and its chunks with Node.js' compile cache (see [the warm run](#the-warm-run)).
- **Off.** The last build found a reason not to bundle the script, and no input changed since then. The run loads each module as without the cache.
- **Miss.** The run loads each module as without the cache and records the run. At exit, it starts a build, which replaces the manifest.

### How a bundle is made

A bundle comes from two processes:

1. **The recorded run** (a miss) runs the script with the normal hooks. A thin wrapper records, for each resolve, the parent, the specifier, `import` or `require()`, the conditions, the URL, and the format, and for each load, the final format. At exit, the run takes the build lock, writes `trace.json`, and starts the builder, detached.
2. **The builder** (`dist/script-bundle-build.js`) starts at the entry. For each ES module, it uses oxc-node's own transform, finds the module's imports, and resolves them from the trace, or with oxc-node's resolver for an import that the run did not make. It selects the modules that the bundle can contain, bundles them with Rolldown into a new `out-<id>/`, and writes `manifest.json` with every input that it read. It then releases the lock.

A module stays out of the bundle and loads from its own file at run time when:

- It is not an ES module: a CommonJS file, a native addon, or a dependency `.js` file in a package without `"type": "module"`, which Node.js can run either way.
- It uses `import.meta` other than `url`, `filename`, `dirname`, `main`, and `resolve()`.
- It uses `require`, `module`, or `exports` without its own declaration. Node.js runs an ES module without them, but Rolldown would give the module working CommonJS ones.
- It uses a direct `eval`, which would see the renamed bindings of the bundle.
- It fails to load or to parse, or one of its static imports does not resolve.
- Code outside the bundle imports or requires it, or it is in `excluded.json`. Its own imports then stay out too, so no module can load twice.

If the builder fails, or if the entry itself must stay out, the manifest gets a `disabled` reason, and later runs are off until an input changes.

### What makes a bundle expire

Every input in the manifest has a stamp: the inode, the size, the modification time, and the change time, or an empty stamp for a path that must not exist. A run compares each stamp exactly. The change time catches an edit that restores the modification time, and the inode catches a file that a rename replaced.

| Input                                                                                                           | Changes that it catches                                                                          |
| --------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Each bundled module, stamped before the builder read it                                                         | An edit to any bundled file, also one made during the build                                      |
| Each import target, also outside the bundle                                                                     | A target that moved or was replaced                                                              |
| The directory of each relative import target outside `node_modules`                                             | A new file that can take the target's place, for example `x.ts` next to an import of `./x.js`    |
| The nearest `package.json` of each module and target, and the project directories between                       | A change to `type`, `exports`, `imports`, or `main`, or a new `package.json` closer to the file  |
| `node_modules/<name>` for each bare specifier, from the importer up to the package found                        | An install, an upgrade, a relink of a workspace package, or a new package closer to the importer |
| The tsconfig files that oxc-node read                                                                           | A change to compiler options or `paths`                                                          |
| `vite-plus`'s `package.json`, the preload, the hooks module, the builder, and the core package's `package.json` | An upgrade or a reinstall of Vite+                                                               |

A change of the key selects another directory, so it never uses a stale bundle. A data file that the script reads at run time is not an input: the bundle contains code, not the script's data.

The run itself can also make a bundle expire. If a module of the bundle also loads from its own file, the run warns, adds the module to `excluded.json`, and removes `manifest.json`. The next run is a miss, and the new build leaves the module out.

A failed build records every file that it read, the project directories of those files, and the tsconfig files. Thus a fix to the file that failed makes the next run a miss, and the build runs again.

### When a bundle is reused

- **Same entry, key, and inputs.** Every run is a hit, with any script arguments. An `import()` that the recorded run did not reach is in the bundle when it resolves at build time, or resolves at run time from its own module (`vpx-from:`).
- **After a change.** One input change makes the whole bundle expire. There is no partial reuse: the next run is a miss at the speed without the cache, and its build bundles all modules again. The run after the build is a hit, but its chunks have new paths, so V8 compiles them again. Later hits also use the compile cache. On Knip, after an edit to one source file, the three runs took 291.3 ms, 218.5 ms, and 139.1 ms.
- **Concurrent runs.** Two recorded runs can end at the same time. The first takes the lock and builds, and the second leaves the build to it. A lock older than 60 seconds belongs to a builder that died, and the next run takes it.
- **Older outputs.** A build keeps the three newest older `out-*` directories of its entry, so a run that started from one of them can still load its `import()` chunks. It removes the rest.

## Design details

### When the cache applies

The preload (`script-preload.ts`) requires `dist/script-bundle-hooks.cjs` only when `VPX_BUNDLE_CACHE=1`, so other runs do not read its code. The cache is off, with a reason under `VPX_BUNDLE_CACHE_DEBUG=1`, in these conditions:

- Watch mode, a worker thread, or a Node.js release without in-thread module hooks (before 22.22.3 and 24.11.1, see `canRegisterSyncHooks`).
- `--require`, `--import`, or `--loader` other than the loader's own entries, in the arguments or in `NODE_OPTIONS`. Their hooks would not see the bundled modules.
- `NODE_V8_COVERAGE`, `--test`, `--eval`, `--check`, the REPL, `--preserve-symlinks`, `NODE_PRESERVE_SYMLINKS`, and a Node.js config file.
- A script with no `node_modules` directory above it.

### Cache location

The warm path must find the cache without the 45 MB native binding, so the cache is in the project's `node_modules`, not in a `VpDirs` root. A script with no `node_modules` directory above it runs without the cache.

Under `vp run`, the loader calls `ignoreInput` and `ignoreOutput` for the cache root. Vite Task records each `stat` as a read, so the input check keeps the source files as task inputs.

### The recorded run

On a miss, the run loads the script with the normal hooks. A thin wrapper records each resolve (parent, specifier, `import` or `require`, conditions, URL, format) and the final format of each load. At exit, the run takes the build lock, writes `trace.json`, and starts `node dist/script-bundle-build.js <dir>`, detached, in the script's working directory and environment. The lock covers only the build, which takes less than a second for Knip, because a recorded run can last any time. A run that finds the lock taken leaves the build to its holder. Under `VPX_BUNDLE_CACHE_DEBUG=1`, the build runs before the process exits, so that tests are deterministic.

### The build

The builder loads the native binding and selects the same tsconfig as the run. Then:

1. **Analyze.** Starting at the entry, it loads each ES module with oxc-node's own `load` (the same transform as the run), or reads a dependency file as published. It parses the code with Rolldown's `parseAst`, and it resolves each specifier from the trace, or with oxc-node's resolver for an import that the run did not make (for example, an `import()` that the run did not reach). A module that fails to load or parse stays out of the bundle.
2. **Select** (`bundledSet`). The bundle may contain the ES modules that the entry reaches through modules that [can be bundled](#how-a-bundle-is-made). Every module that code outside the bundle imports stays out, and so do its own imports, until no edge enters the set from outside. Edges from outside are the analyzed imports of unsafe modules, every recorded `require()`, every recorded `import()` with a specifier that is not in the code, and the modules in `excluded.json`.
3. **Bundle** with Rolldown, through a plugin that serves the analyzed code:
   - Each module keeps its own `import.meta.url`, `filename`, and `dirname` (replaced with literals). `import.meta.main` stays only in the entry.
   - A computed `import(x)`, `import.meta.resolve(x)`, and a literal `import()` that does not resolve at build time become `import("vpx-from:<i>:" + (x))`. The warm hooks resolve `x` from module `<i>`'s own URL.
   - `treeshake.annotations` is off and each module has side effects, so every module body and every `/*#__PURE__*/` call that Node.js would run stays. `keepNames` keeps `Function.prototype.name`.
   - Source maps compose oxc-node's map with Rolldown's, so stack traces show the source files with the same lines and columns. The maps omit `sourcesContent`.
4. **Keep the order** (`startupLayout`). Node.js runs a module's static imports depth-first in source order, then the module. A chunk runs all its imports before any of its code, so a module outside the bundle that a chunk imported directly would run before bundled code that came before it, for example a CommonJS client that reads `process.env` after a bundled module sets it. Thus each external module that startup code imports goes through a bundled proxy (`export * from "<url>"`) in a chunk of its own, and the bundled startup modules between two externals form a chunk. The entry has a chunk of its own and imports these chunks in their run order. A module that only `import()` reaches goes into a chunk that loads with the `import()`. Without externals and lazy modules, the build writes one file.
5. **Refuse what the layout cannot keep.** A startup module other than the entry with a top-level `await` holds its chunk open, and an `import()` that it awaits can need that chunk, or a later one: the bundle would wait for itself. Such a script runs without a bundle.
6. **Record the inputs.** The manifest lists each bundled module, stamped before the builder read it, and what resolution depended on: each import target, the directory of each relative target outside `node_modules`, the nearest `package.json` and the directories in between, the `node_modules` lookups for every bare specifier (the package found and each closer location that must stay empty, also when the specifier resolved outside `node_modules` through a workspace link or a tsconfig `paths` alias), the tsconfig files that oxc-node read, and the loader's own files. A stamp is the inode, size, modification time, and change time. The change time catches an edit that restores the modification time.

If any step fails, the manifest records the reason and the inputs that [a failed build](#what-makes-a-bundle-expire) records.

### The warm run

The preload checks every input (about 3 ms for the 917 inputs of Knip) and that every chunk exists, enables Node.js' compile cache in the cache root (unless `NODE_COMPILE_CACHE` or `NODE_DISABLE_COMPILE_CACHE` is set), and registers hooks that wrap the loader's own hooks:

- The main entry resolves to `entry.mjs`. `process.argv[1]` stays the script.
- Chunks, the loader's own files, built-ins, and JavaScript files in `node_modules` go straight to Node.js. For these, the loader gives the same answer as Node.js.
- Everything else goes to the loader's hooks, which load the native binding on first use. On the Knip workload, the binding does not load.
- A load of a bundled module's own URL means a second instance, for example through a computed `import()` that the recorded run did not make. The run prints a warning, adds the module to `excluded.json`, and removes the manifest, so the next run records again and the build leaves the module out.

## Limitations

- **One run with two instances.** If code that the recorded run did not reach loads a module from its own file (a computed `import()`, or a `require()` of an ES module), and that module imports a bundled module, that run has two instances of the bundled module. The run warns, and later runs are correct.
- **Structured stack traces.** `error.stack` maps to the source files. A library that reads call sites (`Error.prepareStackTrace`, for example `callsites`) to find its caller's file sees the chunk's path.
- **Directory changes.** A new file in a directory that holds a bundled module drops the bundle, even when the file does not change resolution, for example a temporary file that the script writes and removes.
- **Order among lazy modules.** The order of modules that only `import()` reaches is kept less strictly: a module that two `import()` targets share loads in a chunk of its own, and the externals that only such modules import load at the top of their chunk.
- **Top-level `await` in a dependency.** A script whose startup modules other than the entry use a top-level `await` runs without a bundle.
- **Hooks registered at run time.** `module.registerHooks()` in the script itself does not see the bundled modules.
- **Under `vp run`.** The detached builder is a descendant of the task, so the task can wait for it on a cold run.
- **No clean-up.** Old cache entries stay until `node_modules` is removed. A build removes only the older outputs of its own entry, after the three newest.

## Results

Apple M4 Pro, median of 30 runs, interleaved. The cache hits ran with a filled compile cache.

Knip 6.44.2 from source, scanning Marked 18.1.3 (`--production --include files,exports --reporter json`), complete run through the global `vpx`. Every mode gives the same report.

| Node.js | v1.6.0 loader | Cache off | Bundle cache hit | Speedup |
| ------- | ------------: | --------: | ---------------: | ------: |
| 22.23.4 |      358.0 ms |  358.8 ms |         189.0 ms |   1.89x |
| 24.22.2 |      307.4 ms |  306.5 ms |         153.7 ms |   2.00x |
| 26.13.1 |      311.8 ms |  311.2 ms |         165.6 ms |   1.88x |

The bundle contains 577 of the 579 ES modules that the builder analyzed, in 10 chunks: 6 externals keep their place in the run order. The cold run that records the trace takes 349.8 ms, compared with 348.6 ms with the cache off (8 runs each on Node.js 24.22.2, with the cache removed before each run). The detached build takes about 300 ms after the run exits.

The fixtures of `bench/vpx-script.ts`, through the loader without the global `vpx`, on Node.js 24.22.2:

| Fixture                             | Cache off | `VPX_BUNDLE_CACHE=1` |       Speedup |
| ----------------------------------- | --------: | -------------------: | ------------: |
| `hello` (one file)                  |   44.1 ms |              37.0 ms |         1.19x |
| `esm-app` (300 modules)             |   79.5 ms |              43.9 ms |         1.81x |
| `cjs-app` (300 modules)             |   79.3 ms |              44.6 ms |         1.78x |
| `large-esm` (1.8 MB)                |   83.1 ms |              41.8 ms |         1.99x |
| `large-cjs` (CommonJS, not bundled) |   88.1 ms |              89.9 ms | 1.8 ms slower |

A script that the cache cannot bundle pays for the hooks module and the manifest check on each run.

### More projects

Commands from other projects (taze at `ab44082`, tsdown at `e2e174d`, and installed ecosystem-CI clones of dify and evcc), through the loader without the global `vpx` routing, on Node.js 24.22.2, median of 20 interleaved runs. Every command gave the same output with and without the cache (for the `tsdown` build, the same output file).

| Command                                     | What it is                                                                | Modules bundled | Cache off | `VPX_BUNDLE_CACHE=1` |       Speedup |
| ------------------------------------------- | ------------------------------------------------------------------------- | --------------: | --------: | -------------------: | ------------: |
| `scripts/report.ts`                         | A script that imports `@aws-sdk/client-s3`, `rxjs`, `date-fns`, and `zod` |      400 of 400 |  542.0 ms |             100.1 ms |         5.41x |
| Knip, `src/cli.ts`                          | The workload above                                                        |      577 of 579 |  305.0 ms |             149.2 ms |         2.04x |
| taze 21.3.0, `src/cli.ts --help`            | Dependency update CLI, from source                                        |      122 of 124 |  127.7 ms |              82.3 ms |         1.55x |
| tsdown 0.23.7, `src/run.ts --help`          | Library bundler CLI, from source (`-C dev`)                               |      124 of 127 |   59.1 ms |              48.4 ms |         1.22x |
| tsdown 0.23.7, build of a one-file library  | The same CLI, doing a small build                                         |       69 of 127 |   96.2 ms |              80.4 ms |         1.20x |
| Vite+ CLI, `packages/cli/src/bin.ts help`   | This repository's CLI, from source                                        |      296 of 297 |   52.4 ms |              46.8 ms |         1.12x |
| Vite+ CLI, `bin.ts --version`               | The same bundle                                                           |      296 of 297 |   53.9 ms |              49.4 ms |         1.09x |
| dify, `scripts/check-icon-dimensions.ts`    | A one-file check script                                                   |          1 of 1 |   42.0 ms |              37.4 ms |         1.12x |
| evcc, `i18n/check.ts`                       | A one-file check script that reads many JSON files                        |          1 of 1 |   88.4 ms |              84.7 ms |         1.04x |
| Prettier 3.9.10, `bin/prettier.cjs --check` | A CommonJS CLI, not bundled                                               |               — |   95.0 ms |              95.7 ms | 0.7 ms slower |

The gain depends on how many modules the script loads at startup, not on how many the bundle contains:

- The library script loads about 650 modules from many packages. Plain Node.js takes 536.2 ms for it too, and 501.7 ms with its compile cache, so the time comes from loading the modules one at a time.
- The Vite+ CLI's bundle has 296 modules, but `help` loads only about 40 of them. The other modules are in chunks that only other commands import. Most of its time is Node.js startup and its own native binding.
- A one-file script saves only the native loader's startup, about 4–7 ms.
- A script that spends its time on its own work (evcc reads and compares JSON files) gains little.
- A CommonJS entry is never bundled.

## Next steps

- Support the Node.js releases with off-thread hooks only (22.18 to 22.22.2), where the hooks thread would start for each warm run.
- Decide the conditions for a default: real-project coverage, Windows and Linux results, and cache clean-up.
- Report the two per-module costs that #3343 found to oxc-node, independently of this cache: the inline source map includes `sourcesContent`, which makes V8 scan about four times the source size, and `add_short_circuit` resolves each import a second time through Node.js.
