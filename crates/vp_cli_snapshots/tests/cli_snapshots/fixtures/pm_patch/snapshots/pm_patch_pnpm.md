# pm_patch_pnpm

## `vp pm patch placeholder -- --version`

pnpm receives the patch command

```
10.34.4
```

## `vp pm patch-commit placeholder -- --version`

pnpm receives the patch-commit command

```
10.34.4
```

## `vpt json-edit package.json packageManager pnpm@7.3.0`


## `vp pm patch placeholder`

Before pnpm 7.4.0, patch warns and skips execution.

```
warn: pnpm patch requires pnpm >= 7.4.0. Skipping.
```

## `vp pm patch-commit placeholder`

Before pnpm 7.4.0, patch-commit also warns and skips execution.

```
warn: pnpm patch-commit requires pnpm >= 7.4.0. Skipping.
```
