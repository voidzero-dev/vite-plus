# command_dev_bundled_dev

## `npm install --ignore-scripts --legacy-peer-deps --no-audit --no-fund`


## `vp dev --host 127.0.0.1 --port 0`

Bundled dev serves core's runtime without a separate rolldown package.

**Exit code:** 130

**→ expect-milestone:** `dev-server:runtime-verified`

```
VITE+ - The Unified Toolchain for the Web

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/nested/
  ➜  press h + enter to show help
Rolldown package: missing
Client imports /nested/@rolldown/experimental-runtime.mjs
Served runtime matches core
```

**← write-key:** `ctrl-c`

```
VITE+ - The Unified Toolchain for the Web

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/nested/
  ➜  press h + enter to show help
Rolldown package: missing
Client imports /nested/@rolldown/experimental-runtime.mjs
Served runtime matches core

```
