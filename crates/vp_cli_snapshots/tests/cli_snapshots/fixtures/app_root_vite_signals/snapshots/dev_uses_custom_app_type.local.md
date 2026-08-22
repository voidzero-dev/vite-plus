# dev_uses_custom_app_type

A non-static Vite `appType` makes the workspace root runnable for `vp dev`.

## `vpt cp configs/custom-app.ts vite.config.ts`


## `vp dev --host 127.0.0.1 --port 0`

**→ expect-milestone:** `dev-server:ready`

```

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
