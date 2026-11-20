# legacy_cli_at_fixed_target

The global CLI keeps explicit arguments and falls back only when a legacy CLI would select another package.

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


## `vp -C app build --config vite.config.mjs`

```
VITE+ - The Unified Toolchain for the Web

Legacy CLI in app: ["build","--config","vite.config.mjs"]
```

## `vp -C app build -- --help`

```
VITE+ - The Unified Toolchain for the Web

Legacy CLI in app: ["build","--","--help"]
```

## `vpt stat-file app/nested/dist --assert missing`

```
app/nested/dist: missing
```

## `vpt rm app/vite.config.mjs`


## `vp -C app build`

```
VITE+ - The Unified Toolchain for the Web

Legacy CLI in app: ["build"]
```
