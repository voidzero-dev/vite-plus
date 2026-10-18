# dev_framework_note

Framework dependencies without a dev script get a hint. Vite still starts.

## `vp dev`

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Astro and has no dev npm script. `vp dev` starts the Vite dev server. To start the Astro dev server, run `vp exec astro dev` from this package.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Astro and has no dev npm script. `vp dev` starts the Vite dev server. To start the Astro dev server, run `vp exec astro dev` from this package.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vp -C app dev`

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Nuxt and has no dev npm script. `vp dev` starts the Vite dev server. To start the Nuxt dev server, run `vp exec nuxt dev` from this package.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

note: This package depends on Nuxt and has no dev npm script. `vp dev` starts the Vite dev server. To start the Nuxt dev server, run `vp exec nuxt dev` from this package.

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
note: This package depends on Nuxt and has no dev npm script. `vp dev` starts the Vite dev server. To start the Nuxt dev server, run `vp exec nuxt dev` from this package.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

note: vp dev: using ../app (defaultPackage in vite.config.ts)
note: This package depends on Nuxt and has no dev npm script. `vp dev` starts the Vite dev server. To start the Nuxt dev server, run `vp exec nuxt dev` from this package.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
