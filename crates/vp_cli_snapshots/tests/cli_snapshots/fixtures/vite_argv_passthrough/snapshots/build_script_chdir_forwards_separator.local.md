# build_script_chdir_forwards_separator

A task script that starts with vp -C forwards the separator to Vite like a direct call.

## `vp run build:chdir`


## `vpt grep-file vite-argv.json '["build","--","--help"]'`

```
vite-argv.json: found "[\"build\",\"--\",\"--help\"]"
```

## `vpt grep-file dist/index.html 'Build from the project root'`

```
dist/index.html: found "Build from the project root"
```
