# build_and_preview_use_custom_vite_root

A workspace root remains the app target when its static Vite root contains
index.html. Bare vp build creates the app, and vp preview serves its output.

## `vp build`

```
VITE+ - The Unified Toolchain for the Web

✓ 2 modules transformed.
computing gzip size...
src/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp preview --host 127.0.0.1 --port 0`

**→ expect-milestone:** `preview-server:ready`

```
VITE+ - The Unified Toolchain for the Web

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
