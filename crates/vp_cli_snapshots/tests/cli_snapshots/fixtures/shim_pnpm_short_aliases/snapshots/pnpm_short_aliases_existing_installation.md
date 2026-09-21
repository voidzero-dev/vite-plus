# pnpm_short_aliases_existing_installation

## `pnpm --version`


## `node assert-aliases.cjs 10.18.0`

Short aliases reuse an already installed pre-v11 pnpm.

```
pn and pnx use the selected pnpm and dlx binaries
```

## `vp env which pn`

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm
  Package:    pnpm@10.18.0
  Source:     <workspace>/package.json
```

## `vp env which pnx`

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx
  Package:    pnpm@10.18.0
  Source:     <workspace>/package.json
```

## `pn exec node assert-aliases.cjs 10.18.0`

Nested explicit shim calls preserve the inherited package manager.

```
pn and pnx use the selected pnpm and dlx binaries
```

## `VP_PNPM_VERSION=10.19.0 node assert-aliases.cjs 10.19.0`

Both aliases honor the pnpm version override.

```
pn and pnx use the selected pnpm and dlx binaries
```

## `vp env exec --package-manager pnpm@10.19.0 node assert-aliases.cjs 10.19.0`

An explicitly injected version takes precedence over the project pin.

```
pn and pnx use the selected pnpm and dlx binaries
```
