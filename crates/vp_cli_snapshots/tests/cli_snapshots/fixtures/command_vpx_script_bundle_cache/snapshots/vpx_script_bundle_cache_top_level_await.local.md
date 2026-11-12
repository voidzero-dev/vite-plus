# vpx_script_bundle_cache_top_level_await

A dependency that awaits at its top level keeps the script off the bundle, where its chunk could wait for itself. The script runs as without the cache.

## `vpx ./tla-main.ts`

The recorded run; the build reports why it does not bundle

```
vpx bundle cache: miss (no bundle); recording
early=loaded:1
vpx bundle cache: the bundle could not be built: <workspace>/tla.ts awaits at its top level
```

## `vpx ./tla-main.ts`

Later runs load each module, with the same result

```
vpx bundle cache: off (the bundle could not be built: <workspace>/tla.ts awaits at its top level)
early=loaded:1
```
