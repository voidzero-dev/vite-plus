# default_package_value_wrapper

Regression: the TypeScript wrapper is on the defaultPackage VALUE
(`'./frontend' as const`), not the config object. Static extraction must
unwrap it too, so vp builds ./frontend rather than erroring that
defaultPackage is not a static string literal.

## `cd value_wrapper && vp build`

```
VITE+ - The Unified Toolchain for the Web

note: vp build: using ./frontend (defaultPackage)
vite <version> building client environment for production...
✓ 2 modules transformed.
computing gzip size...
dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
