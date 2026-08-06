# default_package_define_config_wrapper

defaultPackage inside `defineConfig({ ... } satisfies UserConfig)` is also
honored: the wrapper on the defineConfig argument is unwrapped too.

## `cd dc_wrapper && vp build`

```
VITE+ - The Unified Toolchain for the Web

note: vp build: using ./frontend (defaultPackage in vite.config.ts)
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
