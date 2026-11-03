# build_and_preview_use_custom_vite_root

The config sets a static Vite `root`. The `index.html` file is in that directory.
Bare `vp build` builds the workspace app. Then `vp preview` serves the build
output.

## `vp build`

```
✓ 2 modules transformed.
computing gzip size...
src/dist/index.html  <size> kB │ gzip: <size> kB

✓ built in <duration>
```

## `vp preview`

**→ expect-milestone:** `preview-server:ready`

```
  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
