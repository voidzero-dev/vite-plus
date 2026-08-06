# synthetic_dev_cache_disabled

## `vp run dev`

synthetic dev (vp dev) should have cache disabled even with cacheScripts

**Exit code:** 130

**→ expect-milestone:** `dev-server:ready`

```
$ vp dev --host 127.0.0.1 --port 0 ⊘ cache disabled

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-key:** `ctrl-c`

```
$ vp dev --host 127.0.0.1 --port 0 ⊘ cache disabled

  VITE+ <version>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help

```
