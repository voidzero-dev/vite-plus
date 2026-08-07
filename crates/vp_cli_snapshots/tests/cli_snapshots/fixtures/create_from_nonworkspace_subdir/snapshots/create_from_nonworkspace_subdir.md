# create_from_nonworkspace_subdir

## `cd scripts && vp create --no-interactive --package-manager pnpm vite:application`

explicit package manager overrides the non-workspace ancestor


## `vpt stat-file scripts/vite-plus-application/package.json --assert file`

created at scripts/vite-plus-application

```
scripts/vite-plus-application/package.json: file
```

## `vpt grep-file scripts/vite-plus-application/package.json '"name": "pnpm"'`

pins pnpm in devEngines

```
scripts/vite-plus-application/package.json: found "\"name\": \"pnpm\""
```

## `vpt stat-file vite-plus-application/package.json --assert-not file`

not created at parent root

```
vite-plus-application/package.json: missing
```
