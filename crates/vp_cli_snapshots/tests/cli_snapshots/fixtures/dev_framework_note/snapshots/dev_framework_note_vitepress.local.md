# dev_framework_note_vitepress

VitePress hints use the detected documentation root and are suppressed for Vite apps.

## `vp -C site dev`

Documentation under docs/ needs a root argument.

**→ expect-milestone:** `dev-server:ready`

```
note: This package depends on VitePress and has no dev script or task. `vp dev` starts the Vite dev server. To start the VitePress dev server, change to `<workspace>/site` and run `vp exec vitepress dev docs`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
note: This package depends on VitePress and has no dev script or task. `vp dev` starts the Vite dev server. To start the VitePress dev server, change to `<workspace>/site` and run `vp exec vitepress dev docs`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vpt cp -r site/docs/.vitepress site/.vitepress`


## `vp -C site dev`

Configuration at the package root takes precedence.

**→ expect-milestone:** `dev-server:ready`

```
note: This package depends on VitePress and has no dev script or task. `vp dev` starts the Vite dev server. To start the VitePress dev server, change to `<workspace>/site` and run `vp exec vitepress dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```
note: This package depends on VitePress and has no dev script or task. `vp dev` starts the Vite dev server. To start the VitePress dev server, change to `<workspace>/site` and run `vp exec vitepress dev`.

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```

## `vpt write-file site/index.html '<div id='\''app'\''></div>'`


## `vp -C site dev`

A Vite app with a documentation dependency gets no framework hint.

**→ expect-milestone:** `dev-server:ready`

```

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
```

**← write-line:** `q`

```

  VITE+ <version>  ready in <duration>

  ➜  Local:   http://127.0.0.1:<port>/
  ➜  press h + enter to show help
q
```
