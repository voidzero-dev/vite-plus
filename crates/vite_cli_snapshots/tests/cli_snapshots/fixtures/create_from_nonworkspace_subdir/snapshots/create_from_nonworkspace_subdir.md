# create_from_nonworkspace_subdir

## `cd scripts && vp create --no-interactive vite:application`

from non-monorepo subdir


## `vpt stat-file scripts/vite-plus-application/package.json --assert file`

created at scripts/vite-plus-application

```
scripts/vite-plus-application/package.json: file
```

## `vpt stat-file vite-plus-application/package.json --assert-not file`

not created at parent root

```
vite-plus-application/package.json: missing
```
