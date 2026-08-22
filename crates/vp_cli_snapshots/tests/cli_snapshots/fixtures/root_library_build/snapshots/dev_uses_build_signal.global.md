# dev_uses_build_signal

The same declared `build` field makes the workspace root runnable for `vp dev`.

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
