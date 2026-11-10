# command_dev_bundled_dev_unrelated_rolldown

## `npm install --ignore-scripts --legacy-peer-deps --no-audit --no-fund`


## `vpt cp -r unrelated-rolldown node_modules/rolldown`


## `vp dev --host 127.0.0.1 --port 0`

An unrelated rolldown package must not replace core's bundled runtime.

**Exit code:** 130

**→ expect-milestone:** `dev-server:runtime-verified`

```
VITE+ - The Unified Toolchain for the Web

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/nested/
  ➜  press h + enter to show help
Rolldown package: unrelated
Client imports /nested/@rolldown/experimental-runtime.mjs
Served runtime matches core
```

**← write-key:** `ctrl-c`

```
VITE+ - The Unified Toolchain for the Web

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/nested/
  ➜  press h + enter to show help
Rolldown package: unrelated
Client imports /nested/@rolldown/experimental-runtime.mjs
Served runtime matches core

```
