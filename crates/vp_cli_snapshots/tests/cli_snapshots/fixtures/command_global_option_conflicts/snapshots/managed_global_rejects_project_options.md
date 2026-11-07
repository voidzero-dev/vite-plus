# managed_global_rejects_project_options

Project-only flags are rejected before managed global operations can install, remove, or update packages.

## `vp install -g -D --frozen-lockfile --filter app typescript`

**Exit code:** 2

```
error: the argument '--global' cannot be used with:
  --dev
  --frozen-lockfile
  --filter <PATTERN>

Usage: vp install --global <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

## `vp add -g --allow-build typescript typescript`

**Exit code:** 2

```
error: the argument '--global' cannot be used with '--allow-build <NAMES>'

Usage: vp add --global <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

## `vp remove -g --filter app typescript`

**Exit code:** 2

```
error: the argument '--global' cannot be used with '--filter <PATTERN>'

Usage: vp remove --global <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

## `vp update -g --interactive typescript`

**Exit code:** 2

```
error: the argument '--global' cannot be used with '--interactive'

Usage: vp update --global <PACKAGES>... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

## `vp outdated -g --compatible`

**Exit code:** 2

```
error: the argument '--global' cannot be used with '--compatible'

Usage: vp outdated --global [PACKAGES]... [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```

## `vp pm list -g --depth 0`

**Exit code:** 2

```
error: the argument '--global' cannot be used with '--depth <DEPTH>'

Usage: vp pm list --global [PATTERN] [-- <PASS_THROUGH_ARGS>...]

For more information, try '--help'.
```
