# config_only_root_build

Regression: a workspace root that builds via vite.config.ts (build.lib, no
index.html) is a valid target. Bare vp build must run the root build in
place, not elicit the members, even with a member present.

## `vp build`

```
vite <version> building client environment for production...
[2Ktransforming...✓ 2 modules transformed.
rendering chunks...
computing gzip size...
dist/index.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
