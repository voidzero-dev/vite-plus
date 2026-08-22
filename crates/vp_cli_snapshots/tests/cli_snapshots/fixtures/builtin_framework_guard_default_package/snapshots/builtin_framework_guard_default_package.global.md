# builtin_framework_guard_default_package

## `vp build`

the root holds a nuxt.config.ts, but defaultPackage retargets the build into apps/web, so it runs

```
VITE+ - The Unified Toolchain for the Web

note: vp build: using ./apps/web (defaultPackage in vite.config.ts)
✓ 4 modules transformed.
computing gzip size...
dist/index.html                <size> kB │ gzip: <size> kB
dist/assets/index-<hash>.js  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vpt list-dir apps/web/dist`

output lands in the configured package

```
assets
index.html
```
