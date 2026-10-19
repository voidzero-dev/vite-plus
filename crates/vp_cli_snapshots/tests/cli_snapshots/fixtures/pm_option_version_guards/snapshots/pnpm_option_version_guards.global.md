# pnpm_option_version_guards

## `vp add testnpm2 --allow-build testnpm2`

**Exit code:** 1

```
pnpm < 10.4.0 does not support --allow-build.
```

## `vp add testnpm2 --save-catalog-name web`

**Exit code:** 1

```
pnpm < 10.12.0 does not support --save-catalog-name.
```

## `vp install testnpm2 --save-catalog`

The install alias must use the add support checks.

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

pnpm < 10.12.0 does not support --save-catalog.
```

## `vp pm pack --recursive`

**Exit code:** 1

```
pnpm < 10.11.0 does not support --recursive.
```

## `vp pm list --find-by finder`

**Exit code:** 1

```
pnpm < 10.16.0 does not support --find-by.
```

## `vp why testnpm2 --find-by finder`

**Exit code:** 1

```
pnpm < 10.16.0 does not support --find-by.
```

## `vp why --find-by finder`

Package names remain required even when a finder is supplied.

**Exit code:** 2

```
error: the following required arguments were not provided:
  <PACKAGES>...

Usage: vp why --find-by <FINDER_NAME> <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```
