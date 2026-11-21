# vpx_script_bundle_cache_order

Modules outside the bundle run where they ran without it: a CommonJS dependency that reads the environment while it loads sees what a bundled module before it set.

## `vpx ./env-main.ts`

The recorded run

```
vpx bundle cache: miss (no bundle); recording
url=from-load-env order=load-env,reads-env,after
vpx bundle cache: bundled 3 of 3 ES modules in <duration>
```

## `vpx ./env-main.ts`

The bundled run keeps the order

```
vpx bundle cache: hit (3 modules)
url=from-load-env order=load-env,reads-env,after
vpx bundle cache: the native loader did not load
```
