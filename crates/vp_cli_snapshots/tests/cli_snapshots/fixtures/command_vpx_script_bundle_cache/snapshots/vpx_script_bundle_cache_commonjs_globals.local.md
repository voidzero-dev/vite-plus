# vpx_script_bundle_cache_commonjs_globals

An ES module that uses `require` without declaring it stays out of the bundle: Rolldown would give it a working `require()`, which Node.js does not.

## `vpx ./globals-main.ts`

The recorded run; the build leaves the dependency out

```
vpx bundle cache: miss (no bundle); recording
path=catch
vpx bundle cache: left out <workspace>/node_modules/uses-require/index.js: it uses require, which an ES module does not have
vpx bundle cache: bundled 1 of 2 ES modules in <duration>
```

## `vpx ./globals-main.ts`

The bundled run takes the same path

```
vpx bundle cache: hit (1 modules)
path=catch
vpx bundle cache: the native loader did not load
```
