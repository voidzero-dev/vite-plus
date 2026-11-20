# legacy_cli_at_fixed_target

The global CLI builds the fixed directory with that project's Node.js when its local CLI predates the fixed target marker.

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


## `vp build`


## `vpt print-file app/dist/index.html`

```
<h1>Selected app</h1>
```

## `vpt print-file app/build-args.json`

```
["build"]
```

## `vpt rm -r app/dist`


## `vp -C app build`


## `vpt print-file app/dist/index.html`

```
<h1>Selected app</h1>
```

## `vpt rm -r app/dist`


## `vp -C app build -- --help`


## `vpt stat-file app/dist/index.html --assert file`

```
app/dist/index.html: file
```

## `vpt print-file app/build-args.json`

```
["build","--","--help"]
```

## `vpt stat-file app/nested/dist --assert missing`

```
app/nested/dist: missing
```
