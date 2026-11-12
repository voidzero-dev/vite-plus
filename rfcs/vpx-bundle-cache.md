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

## Design

### When the cache applies

The preload (`script-preload.ts`) requires `dist/script-bundle-hooks.cjs` only when `VPX_BUNDLE_CACHE=1`, so other runs do not read its code. The cache is off, with a reason under `VPX_BUNDLE_CACHE_DEBUG=1`, in these conditions:

- Watch mode, a worker thread, or a Node.js release without in-thread module hooks (before 22.22.3 and 24.11.1, see `canRegisterSyncHooks`).
- `--require`, `--import`, or `--loader` other than the loader's own entries, in the arguments or in `NODE_OPTIONS`. Their hooks would not see the bundled modules.
- `NODE_V8_COVERAGE`, `--test`, `--eval`, `--check`, the REPL, `--preserve-symlinks`, `NODE_PRESERVE_SYMLINKS`, and a Node.js config file.
- A script with no `node_modules` directory above it.

### Cache location and key

The cache is in the nearest `node_modules/.vite-plus/vpx-bundle/` above the script, one directory for each entry and key. The warm path must find it without the 45 MB native binding, so it does not use `VpDirs`. The key holds the Node.js version, platform, architecture, the entry's real path, the working directory, `execArgv`, `NODE_OPTIONS`, and the loader's tsconfig variables. The manifest stores the full key, and a different key is a miss.

Under `vp run`, the loader calls `ignoreInput` and `ignoreOutput` for the cache root. Vite Task records each `stat` as a read, so the input check below keeps the source files as task inputs.

### The recorded run

On a miss, the run loads the script with the normal hooks. A thin wrapper records each resolve (parent, specifier, `import` or `require`, conditions, URL, format) and the final format of each load. At exit, the run takes the build lock, writes `trace.json`, and starts `node dist/script-bundle-build.js <dir>`, detached, in the script's working directory and environment. The lock covers only the build, which takes less than a second for Knip, because a recorded run can last any time. A run that finds the lock taken leaves the build to its holder. Under `VPX_BUNDLE_CACHE_DEBUG=1`, the build runs before the process exits, so that tests are deterministic.

### The build

The builder loads the native binding and selects the same tsconfig as the run. Then:

1. **Analyze.** Starting at the entry, it loads each ES module with oxc-node's own `load` (the same transform as the run), or reads a dependency file as published. It parses the code with Rolldown's `parseAst`, and it resolves each specifier from the trace, or with oxc-node's resolver for an import that the run did not make (for example, an `import()` that the run did not reach). A module that fails to load or parse stays out of the bundle.
2. **Select** (`bundledSet`). The bundle may contain the ES modules that the entry reaches through safe modules. Every module that code outside the bundle imports stays out, and so do its own imports, until no edge enters the set from outside. Edges from outside are the analyzed imports of unsafe modules, every recorded `require()`, every recorded `import()` with a specifier that is not in the code, and the modules in `excluded.json`.
3. **Bundle** with Rolldown, through a plugin that serves the analyzed code:
   - Each module keeps its own `import.meta.url`, `filename`, and `dirname` (replaced with literals). `import.meta.main` stays only in the entry.
   - A computed `import(x)`, `import.meta.resolve(x)`, and a literal `import()` that does not resolve at build time become `import("vpx-from:<i>:" + (x))`. The warm hooks resolve `x` from module `<i>`'s own URL.
   - `treeshake.annotations` is off and each module has side effects, so every module body and every `/*#__PURE__*/` call that Node.js would run stays. `keepNames` keeps `Function.prototype.name`.
   - Source maps compose oxc-node's map with Rolldown's, so stack traces show the source files with the same lines and columns. The maps omit `sourcesContent`.
4. **Keep the order** (`startupLayout`). Node.js runs a module's static imports depth-first in source order, then the module. A chunk runs all its imports before any of its code, so a module outside the bundle that a chunk imported directly would run before bundled code that came before it, for example a CommonJS client that reads `process.env` after a bundled module sets it. Thus each external module that startup code imports goes through a bundled proxy (`export * from "<url>"`) in a chunk of its own, and the bundled startup modules between two externals form a chunk. The entry has a chunk of its own and imports these chunks in their run order. A module that only `import()` reaches goes into a chunk that loads with the `import()`. Without externals and lazy modules, the build writes one file.
5. **Refuse what the layout cannot keep.** A startup module other than the entry with a top-level `await` holds its chunk open, and an `import()` that it awaits can need that chunk, or a later one: the bundle would wait for itself. Such a script runs without a bundle.
6. **Record the inputs.** The manifest lists each bundled module, stamped before the builder read it, and what resolution depended on: each import target, the directory of each relative target outside `node_modules`, the nearest `package.json` and the directories in between, the `node_modules` lookups for every bare specifier (the package found and each closer location that must stay empty, also when the specifier resolved outside `node_modules` through a workspace link or a tsconfig `paths` alias), the tsconfig files that oxc-node read, and the loader's own files. A stamp is the inode, size, modification time, and change time. The change time catches an edit that restores the modification time.

If any step fails, the manifest records the reason, and the script runs without a bundle until an input changes. The inputs of a failed build are every file the builder read, the project directories of those files, and the tsconfig files, so a fix to the file that failed brings the bundle back.

A build keeps the three newest older outputs of its entry, so a run that started from one of them can still load its `import()` chunks, and removes the rest.

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

## Next steps

- Support the Node.js releases with off-thread hooks only (22.18 to 22.22.2), where the hooks thread would start for each warm run.
- Decide the conditions for a default: real-project coverage, Windows and Linux results, and cache clean-up.
- Report the two per-module costs that #3343 found to oxc-node, independently of this cache: the inline source map includes `sourcesContent`, which makes V8 scan about four times the source size, and `add_short_circuit` resolves each import a second time through Node.js.
