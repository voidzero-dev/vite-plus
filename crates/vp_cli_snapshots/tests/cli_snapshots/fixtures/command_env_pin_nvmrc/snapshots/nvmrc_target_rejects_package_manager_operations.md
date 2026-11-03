# nvmrc_target_rejects_package_manager_operations

## `vp env pin pnpm@10.18.0 --target nvmrc --no-install`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Node.js file targets cannot pin a package manager
```

## `vp env pin 22.13.0 pnpm@10.18.0 --target nvmrc --no-install`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: mixed Node.js and package-manager pins require the default targets or --target dev-engines
```

## `vp env unpin pm --target nvmrc`

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Node.js file targets are incompatible with package-manager scope
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
