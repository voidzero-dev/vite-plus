# dev_framework_note

Framework dependencies without a dev script or task get a hint. Vite still starts.

## `vpt cp -r app 'app with spaces'`


## `vp dev`

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Astro and has no dev script or task. `vp dev` starts the Vite dev server. To start the Astro dev server, run `vp exec astro dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Astro and has no dev script or task. `vp dev` starts the Vite dev server. To start the Astro dev server, run `vp exec astro dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vp -C app dev`

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `cd default-package && vp dev`

The hint uses the resolved defaultPackage, which depends on Nuxt, not the caller's Astro dependency.

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

note: vp dev: using ../app (defaultPackage in vite.config.ts)
note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

note: vp dev: using ../app (defaultPackage in vite.config.ts)
note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vp -C 'app with spaces' dev`

The directory instruction identifies paths with spaces without requiring shell-specific quoting.

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app with spaces` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app with spaces` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
