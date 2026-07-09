# explicit_config_forwards

An explicit -c/--config file is explicit build intent: vp forwards it to
Vite instead of eliciting a package. Bare vp build here would elicit (no
runnable root, a member present), but -c lib.config.ts builds the lib.

## `vp build -c lib.config.ts`

```
vite <version> building client environment for production...
[2Ktransforming...✓ 2 modules transformed.
rendering chunks...
computing gzip size...
dist/lib.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
