# preview_uses_build_output

A static `build.outDir` with `index.html` makes the workspace root runnable for `vp preview`.

## `vpt cp configs/preview-output.ts vite.config.ts`


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
