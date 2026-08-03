# pm_patch_yarn_berry

## `vp pm patch placeholder`

Yarn Berry receives the patch command and rejects the uninstalled package

**Exit code:** 1

```
Usage Error: No package found in the project for the given locator

$ yarn patch [-u,--update] [--json] <package>
```

## `vp pm patch-commit placeholder`

Yarn Berry receives the patch-commit command and rejects the missing patch folder

**Exit code:** 1

```
Usage Error: The argument folder didn't get created by 'yarn patch'

$ yarn patch-commit [-s,--save] <patchFolder>
```
