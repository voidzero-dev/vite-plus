# default_package_ts_wrapper

Regression: defaultPackage declared through a TypeScript wrapper
(`satisfies UserConfig`, `as const`) is still a static string literal and
must be honored. vp builds ./frontend, not the wrapper root.

## `cd ts_wrapper && vp build`

```
note: vp build: using ./frontend (defaultPackage)
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
