# dev_framework_note_selected_script

A selected package with a dev script gets one script hint that names its directory.

## `vpt json-edit app/package.json scripts.dev 'nuxt dev'`


## `cd default-package && vp dev`

**→ expect-milestone:** `dev-server:ready`

```
note: vp dev: using ../app (defaultPackage in vite.config.ts)
note: You are running `vp dev` as a Vite+ built-in command. If you meant to run the dev npm script, change to `<workspace>/app` and use `vpr dev` instead.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
note: vp dev: using ../app (defaultPackage in vite.config.ts)
note: You are running `vp dev` as a Vite+ built-in command. If you meant to run the dev npm script, change to `<workspace>/app` and use `vpr dev` instead.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vp -C app dev`

**→ expect-milestone:** `dev-server:ready`

```
note: You are running `vp dev` as a Vite+ built-in command. If you meant to run the dev npm script, change to `<workspace>/app` and use `vpr dev` instead.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
note: You are running `vp dev` as a Vite+ built-in command. If you meant to run the dev npm script, change to `<workspace>/app` and use `vpr dev` instead.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
