# env_pin_package_manager_requires_confirmation

## `vpt pipe-stdin 'n
' -- vp env pin yarn@4.12.0 --no-install`

declining the overwrite prompt preserves the existing package-manager pin

```
VITE+ - The Unified Toolchain for the Web

warn: Current environment resolves to pnpm from packageManager, but yarn was requested.
Package manager already pinned to pnpm@10.18.0
Overwrite with yarn@4.12.0? (Y/n): Cancelled.
```

## `vpt print-file package.json`

package.json was not rewritten

```
{
  "name": "command-env-package-manager-mismatch",
  "private": true,
  "packageManager": "pnpm@10.18.0"
}
```
