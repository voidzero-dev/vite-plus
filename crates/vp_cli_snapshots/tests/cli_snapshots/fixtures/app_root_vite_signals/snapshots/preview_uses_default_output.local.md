# preview_uses_default_output

A default preview output makes the workspace root runnable without another Vite signal.

## `vpt cp configs/preview-default-output.ts vite.config.ts`


## `vpt mkdir dist`


## `vpt cp output/index.html dist/index.html`


## `vp preview --host 127.0.0.1 --port 0`

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
