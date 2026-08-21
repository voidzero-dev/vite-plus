# command_prune_bun

## `vp install -- --silent`

should install packages first

```
VITE+ - The Unified Toolchain for the Web
```

## `vp pm prune`

should prune extraneous dependencies

```
bun prune <version> (<hash>)

Done! Checked 2 packages across 1 folder (nothing to prune) [<duration>]
```

## `vp pm prune --prod`

should prune dev dependencies

```
bun prune <version> (<hash>)

- test-vite-plus-package@1.0.0
1 package removed (checked 2) [<duration>]
```

## `vp pm prune --no-optional`

should warn because bun prune has no optional flag

```
warn: bun does not support --no-optional.
bun prune <version> (<hash>)

Done! Checked 1 package across 1 folder (nothing to prune) [<duration>]
```
