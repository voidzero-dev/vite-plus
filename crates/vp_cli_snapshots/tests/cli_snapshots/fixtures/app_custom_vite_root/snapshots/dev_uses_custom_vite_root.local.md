# dev_uses_custom_vite_root

A workspace root remains the app target when its static Vite root points to
the directory that contains index.html. Bare vp dev starts there instead of
eliciting a member package.

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
