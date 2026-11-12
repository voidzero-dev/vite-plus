# vpx_script_bundle_cache_top_level_await

A dependency that awaits an import() at its top level, whose target imports a module it shares with the startup code, does not wait for itself in the bundle.

## `vpx ./tla-main.ts`

The recorded run

```
vpx bundle cache: miss (no bundle); recording
early=loaded:1
vpx bundle cache: bundled 4 of 4 ES modules in <duration>
```

## `vpx ./tla-main.ts`

The bundled run gives the same result

```
vpx bundle cache: hit (4 modules)
early=loaded:1
vpx bundle cache: the native loader did not load
```
