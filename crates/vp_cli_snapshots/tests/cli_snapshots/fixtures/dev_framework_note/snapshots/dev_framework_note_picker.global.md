# dev_framework_note_picker

The hint identifies the package selected in the workspace picker.

## `vpt write-file pnpm-workspace.yaml 'packages:
  - app
  - plain
'`


## `vp dev`

**→ expect-milestone:** `package-select::0`

```
VITE+ - The Unified Toolchain for the Web

Select a package to dev (↑/↓, Enter to run, type to search):

  › nuxt-app       app
    plain-app      plain
    framework-note .
```

**← write:** `nuxt-app`

**→ expect-milestone:** `package-select:nuxt-app:0`

```
VITE+ - The Unified Toolchain for the Web

Select a package to dev (↑/↓, Enter to run, type to search): nuxt-app

  › nuxt-app app
```

**← write-key:** `enter`

**→ expect-milestone:** `dev-server:ready`

```
VITE+ - The Unified Toolchain for the Web

Selected package: nuxt-app (app)
Tip: run this directly with `vp -C app dev`
note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
VITE+ - The Unified Toolchain for the Web

Selected package: nuxt-app (app)
Tip: run this directly with `vp -C app dev`
note: This package depends on Nuxt and has no dev script or task. `vp dev` starts the Vite dev server. To start the Nuxt dev server, change to `<workspace>/app` and run `vp exec nuxt dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
