# vpx_script_bundle_cache_unsupported

Options that load other modules or hooks, or change how the script runs, turn the cache off.

## `vpx --import ./shared.ts ./main.ts`

Another --import preload could register hooks that the bundle would bypass

```
vpx bundle cache: off (--import loads other modules or hooks)
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
```

## `NODE_V8_COVERAGE=${workspace}/coverage vpx ./main.ts`

Coverage reports source files, not a bundle

```
vpx bundle cache: off (NODE_V8_COVERAGE is set)
color=red items=1 esm=esm@1.0.0 cjs=cjs:dep-cjs json=fixture-data
main=true filename=true dirname=workspace class=Store
```
