# create_from_monorepo_subdir

## `cd apps/website && vp create --no-interactive vite:generator`

from workspace subdir


## `vpt stat-file tools/vite-plus-generator/package.json --assert file`

created at tools/vite-plus-generator, not apps/website/

```
tools/vite-plus-generator/package.json: file
```

## `cd apps/website && vp create --no-interactive --git vite:generator`

--git is unavailable for monorepo package create

**Exit code:** 1

```
The --git/--no-git options are not available when adding a package to an existing monorepo
```

## `cd apps/website && vp create --no-interactive --no-git vite:generator`

--no-git is unavailable for monorepo package create

**Exit code:** 1

```
The --git/--no-git options are not available when adding a package to an existing monorepo
```

## `cd apps/website && vp create --no-interactive --git vite:application`

--git is unavailable for monorepo package create

**Exit code:** 1

```
The --git/--no-git options are not available when adding a package to an existing monorepo
```

## `cd apps/website && vp create --no-interactive --no-git vite:library`

--no-git is unavailable for monorepo package create

**Exit code:** 1

```
The --git/--no-git options are not available when adding a package to an existing monorepo
```

## `vpt stat-file apps/website/tools/vite-plus-generator/package.json --assert-not file`

not created inside apps/website/

```
apps/website/tools/vite-plus-generator/package.json: missing
```

## `cd apps && vp create --no-interactive vite:application`

from workspace parent dir


## `vpt stat-file apps/vite-plus-application/package.json --assert file`

created at apps/vite-plus-application

```
apps/vite-plus-application/package.json: file
```

## `cd scripts/helper && vp create --no-interactive vite:library`

from non-workspace dir


## `vpt stat-file packages/vite-plus-library/package.json --assert file`

created at packages/vite-plus-library

```
packages/vite-plus-library/package.json: file
```

## `vpt stat-file scripts/helper/packages/vite-plus-library/package.json --assert-not file`

not created inside scripts/helper/

```
scripts/helper/packages/vite-plus-library/package.json: missing
```

## `cd scripts/helper && vp create --no-interactive vite:application --directory apps/custom-app`

--directory from non-workspace dir


## `vpt stat-file apps/custom-app/package.json --assert file`

created at apps/custom-app with --directory

```
apps/custom-app/package.json: file
```

## `vpt mkdir -p apps/dot-test`

--directory . from monorepo subdir


## `cd apps/dot-test && vp create --no-interactive vite:application --directory .`


## `vpt stat-file apps/dot-test/package.json --assert file`

created at apps/dot-test with --directory .

```
apps/dot-test/package.json: file
```

## `vp create --no-interactive vite:application --directory .`

--directory . from monorepo root should fail

**Exit code:** 1

```
Cannot scaffold into the monorepo root directory. Use --directory to specify a target directory
```

## `vpt mkdir -p apps/website/src`

--directory . inside existing package should fail


## `cd apps/website/src && vp create --no-interactive vite:application --directory .`

**Exit code:** 1

```
Cannot scaffold inside existing package "website" (apps/website). Use --directory to specify a different location
```

## `cd apps/website && vp create --no-interactive vite:application --directory .`

--directory . at existing package root should fail

**Exit code:** 1

```
Cannot scaffold inside existing package "website" (apps/website). Use --directory to specify a different location
```
