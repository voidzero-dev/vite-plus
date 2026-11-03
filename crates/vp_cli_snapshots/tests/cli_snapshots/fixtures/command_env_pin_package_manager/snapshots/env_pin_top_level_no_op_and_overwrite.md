# env_pin_top_level_no_op_and_overwrite

## `vpt write-file package.json '{"packageManager": "pnpm@10.18.0+sha512.keep"}
'`


## `vp env pin pnpm@10.18.0 --no-install`

same-value pin preserves the integrity suffix and reports no changes

```
VITE+ - The Unified Toolchain for the Web

Already pinned to pnpm@10.18.0. No changes made.
  File: package.json
  Field: packageManager
```

## `vpt print-file package.json`

the no-op preserves the original declaration

```
{"packageManager": "pnpm@10.18.0+sha512.keep"}
```

## `vpt pipe-stdin 'y
' -- vp env pin pnpm@10.19.0 --no-install`

the target precedes confirmation and follows success

```
VITE+ - The Unified Toolchain for the Web

  File: package.json
  Field: packageManager
Package manager already pinned to pnpm@10.18.0
Overwrite with pnpm@10.19.0? (Y/n): ✓ Pinned package manager to pnpm@10.19.0
  File: package.json
  Field: packageManager
note: Package manager will be downloaded on first use.
```

## `vpt print-file package.json`

```
{
  "packageManager": "pnpm@10.19.0"
}
```
