# dev_uses_configured_root_without_index

A non-static Vite `root` makes the workspace root runnable without `index.html`.

## `vpt cp configs/root-no-index.ts vite.config.ts`


## `vp dev --host 127.0.0.1 --port 0`

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
