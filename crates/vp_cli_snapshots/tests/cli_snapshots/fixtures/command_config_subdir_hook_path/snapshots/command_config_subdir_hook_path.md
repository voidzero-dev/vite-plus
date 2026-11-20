# command_config_subdir_hook_path

## `git init`


## `cd pkg && vp config --no-agent --hooks-dir .vite-hooks`


## `git config --local core.hooksPath`

the hooks path stays relative to the checkout

```
pkg/.vite-hooks/_
```

## `vpt mkdir -p shadow/pkg`


## `vpt cp -r test-deps pkg/node_modules`


## `git hook run pre-commit`

a package-only binary stays available after cd

```
hook-path-ok
CDPATH=unset
```

## `CDPATH= git hook run pre-commit`

an empty CDPATH remains empty in the user hook

```
hook-path-ok
CDPATH=
```

## `CDPATH=. git hook run pre-commit`

cd must not contaminate the resolved path with extra output

```
hook-path-ok
CDPATH=.
```

## `CDPATH=shadow git hook run pre-commit`

cd must not select shadow/pkg; the user hook still inherits CDPATH

```
hook-path-ok
CDPATH=shadow
```
