# vite_options_and_defaults

Dev and build still accept bare calls and apply Vite options before the separator.

## `vp dev`


## `vpt grep-file vite-argv.json '["dev"]'`

```
vite-argv.json: found "[\"dev\"]"
```

## `vp build`


## `vpt grep-file vite-argv.json '["build"]'`

```
vite-argv.json: found "[\"build\"]"
```

## `vpt grep-file dist/index.html 'Build from the project root'`

```
dist/index.html: found "Build from the project root"
```

## `vp dev --mode staging -- --app web`


## `vpt grep-file vite-argv.json '["dev","--mode","staging","--","--app","web"]'`

```
vite-argv.json: found "[\"dev\",\"--mode\",\"staging\",\"--\",\"--app\",\"web\"]"
```

## `vp build --outDir custom-dist -- --app web --label 'two words'`


## `vpt grep-file vite-argv.json '["build","--outDir","custom-dist","--","--app","web","--label","two words"]'`

```
vite-argv.json: found "[\"build\",\"--outDir\",\"custom-dist\",\"--\",\"--app\",\"web\",\"--label\",\"two words\"]"
```

## `vpt grep-file custom-dist/index.html 'Build from the project root'`

```
custom-dist/index.html: found "Build from the project root"
```
