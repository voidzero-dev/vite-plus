# declining_nvmrc_overwrite_keeps_the_existing_setup

## `vpt pipe-stdin 'n
' -- vp env pin 22.13.0 --no-install`

```
VITE+ - The Unified Toolchain for the Web

.nvmrc already exists with version 20.18.0
Overwrite with 22.13.0? (Y/n): Cancelled.
```

## `vpt print-file .nvmrc package.json`

```
# Node for local tools and CI
<version> # keep this comment
{
  "name": "env-pin-nvmrc",
  "private": true
}
```
