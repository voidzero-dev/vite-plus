# fixed_target_build_artifacts

The bundled CLI builds the selected app when a legacy or preview CLI would select another package.

## `vpt mkdir -p app/node_modules/vite-plus/dist`


## `vpt write-file app/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.2.9","type":"module"}'`


## `vpt cp legacy-probe.mjs app/node_modules/vite-plus/dist/bin.js`


## `vpt cp fixed-target-config.mjs app/vite.config.mjs`


## `vpt write-file app/pnpm-workspace.yaml 'packages:
  - nested
'`


## `vpt write-file app/nested/package.json '{"name":"nested","private":true}'`


## `vpt write-file app/nested/index.html '<h1>Nested</h1>
'`


## `vpt write-file app/index.html '<h1>Selected app</h1>
'`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${PATH} vp build`


## `vpt print-file app/dist/index.html`

```
<h1>Selected app</h1>
```

## `vpt print-file app/build-args.json`

```
["build"]
```

## `vpt rm -r app/dist`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${PATH} vp -C app build`


## `vpt print-file app/dist/index.html`

```
<h1>Selected app</h1>
```

## `vpt rm -r app/dist`


## `vpt write-file app/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.0.0-commit.preview","type":"module"}'`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${PATH} vp -C app build`


## `vpt print-file app/dist/index.html`

```
<h1>Selected app</h1>
```

## `vpt stat-file app/nested/dist --assert missing`

```
app/nested/dist: missing
```
