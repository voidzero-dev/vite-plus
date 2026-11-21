# vpx_script_bundle_cache

VPX_BUNDLE_CACHE=1 records a run, bundles the script's ES modules, and runs the bundle until an input changes. The debug lines show each decision; the builder runs before exit under VPX_BUNDLE_CACHE_DEBUG=1.

## `vpx ./main.ts`

The first run loads each module and records the run; the builder then bundles its ES modules

```
vpx bundle cache: miss (no bundle); recording
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
vpx bundle cache: bundled 6 of 6 ES modules in <duration>
```

## `vpx ./main.ts`

Later runs load the bundle without the native loader; import.meta, CommonJS and ES module dependencies, and JSON behave as before

```
vpx bundle cache: hit (6 modules)
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
vpx bundle cache: the native loader did not load
```

## `vpx ./main.ts lazy`

A literal import() the recorded run did not reach runs from the bundle, with one instance of shared.ts

```
vpx bundle cache: hit (6 modules)
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
lazy=loaded counter=2
vpx bundle cache: the native loader did not load
```

## `vpx ./main.ts throw`

Stack traces map to the source files

**Exit code:** 1

```
vpx bundle cache: hit (6 modules)
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
vpx bundle cache: the native loader did not load
<workspace>/lib.ts:18
    throw new Error(`store ${this.name} failed`);
          ^

Error: store main failed
    at Store.fail (<workspace>/lib.ts:18:11)
    at <anonymous> (<workspace>/main.ts:32:9)
    at ModuleJob.run (node:internal/modules/esm/module_job:343:25)
    at async onImport.tracePromise.__proto__ (node:internal/modules/esm/loader:681:26)
    at async asyncRunEntryPointWithESMLoader (node:internal/modules/run_main:117:5)

Node.js <version>
```

## `vpt replace-file-content lib.ts 'Red = '\''red'\''' 'Red = '\''crimson'\'''`


## `vpx ./main.ts`

An edited source file drops the bundle

```
vpx bundle cache: miss (<workspace>/lib.ts changed); recording
color=crimson items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
vpx bundle cache: bundled 6 of 6 ES modules in <duration>
```

## `vpx ./main.ts`

The rebuilt bundle has the edit

```
vpx bundle cache: hit (6 modules)
color=crimson items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
vpx bundle cache: the native loader did not load
```

## `VPX_BUNDLE_CACHE=0 vpx ./main.ts`

Without VPX_BUNDLE_CACHE=1, the script runs as before

```
color=crimson items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
```
