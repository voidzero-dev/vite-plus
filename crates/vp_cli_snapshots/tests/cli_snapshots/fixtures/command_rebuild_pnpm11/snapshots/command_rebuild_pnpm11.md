# command_rebuild_pnpm11

## `vp rebuild --help`

should show help with [PACKAGES]... positional

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pm rebuild [PACKAGES]... [-- <PASS_THROUGH_ARGS>...]

Rebuild native modules

Arguments:
  [PACKAGES]...           Packages to rebuild (rebuilds all if omitted)
  [PASS_THROUGH_ARGS]...  Additional arguments

Options:
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/install
```

## `vp install`

set up node_modules

```
VITE+ - The Unified Toolchain for the Web

dependencies:
 testnpm2 1.0.0 (1.0.1 is available)

Done in <duration> using pnpm <version>
```

## `vp rebuild`

bare rebuild, no args

```
```

## `vp rebuild testnpm2`

should accept positional package name

```
```

## `vp rebuild testnpm2 -- --recursive`

package name + pass-through args

```
```
