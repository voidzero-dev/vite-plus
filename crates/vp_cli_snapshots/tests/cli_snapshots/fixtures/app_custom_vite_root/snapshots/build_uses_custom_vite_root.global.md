# build_uses_custom_vite_root

A workspace root remains the app target when its static Vite root contains
index.html. Bare vp build creates the app instead of eliciting a member package.

## `vp build`

```
VITE+ - The Unified Toolchain for the Web

✓ 2 modules transformed.
computing gzip size...
src/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```
