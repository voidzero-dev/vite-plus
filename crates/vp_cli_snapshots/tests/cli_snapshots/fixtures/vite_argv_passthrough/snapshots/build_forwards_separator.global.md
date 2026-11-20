# build_forwards_separator

Build forwards the separator and custom arguments, and builds the current directory's entry.

## `vp build -- --app web`


## `vpt grep-file vite-argv.json '["build","--","--app","web"]'`

```
vite-argv.json: found "[\"build\",\"--\",\"--app\",\"web\"]"
```

## `vpt grep-file dist/index.html 'Build from the project root'`

```
dist/index.html: found "Build from the project root"
```
