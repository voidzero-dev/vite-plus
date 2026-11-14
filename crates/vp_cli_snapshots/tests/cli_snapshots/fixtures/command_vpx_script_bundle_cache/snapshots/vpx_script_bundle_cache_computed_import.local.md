# vpx_script_bundle_cache_computed_import

A computed import() resolves from its own module. A module that only it loads, and that imports a bundled module, makes a second instance once; the run warns, and the next build leaves the bundled module out.

## `vpx ./main.ts`

The recorded run does not reach the computed import()

```
vpx bundle cache: miss (no bundle); recording
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
vpx bundle cache: bundled 6 of 6 ES modules in <duration>
```

## `vpx ./main.ts computed`

computed.ts loads from its own file and imports shared.ts from its file too: two instances, so counter=1

```
vpx bundle cache: hit (6 modules)
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
vpx: warning: <workspace>/shared.ts is in the bundle cache but also loaded from its own file, so it has two instances in this run; the next run rebuilds the cache without it
computed=loaded counter=1
resolve=true
vpx bundle cache: the native loader loaded
```

## `vpx ./main.ts computed`

The next run records again, and the build leaves shared.ts out

```
vpx bundle cache: miss (no bundle); recording
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
computed=loaded counter=11
resolve=true
vpx bundle cache: bundled 5 of 6 ES modules in <duration>
```

## `vpx ./main.ts computed`

From then on, shared.ts has one instance

```
vpx bundle cache: hit (5 modules)
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
computed=loaded counter=11
resolve=true
vpx bundle cache: the native loader loaded
```
