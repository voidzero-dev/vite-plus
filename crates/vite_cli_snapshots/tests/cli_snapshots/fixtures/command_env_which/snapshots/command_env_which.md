# command_env_which

## `vp remove -g corepack`

**Exit code:** 1

```
Failed to uninstall corepack: Package corepack is not installed
```

## `vp env exec node --version`

Ensure Node.js is installed first

```
<version>
```

## `vp env which node`

Core tool - shows resolved Node.js binary path

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/js_runtime/node/<version>/bin/node
  Version:    20.18.0
  Source:     <workspace>/.node-version
```

## `vp env which npm`

Core tool - shows resolved npm binary path

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/js_runtime/node/<version>/bin/npm
  Version:    20.18.0
  Source:     <workspace>/.node-version
```

## `vp env which npx`

Core tool - shows resolved npx binary path

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/js_runtime/node/<version>/bin/npx
  Version:    20.18.0
  Source:     <workspace>/.node-version
```

## `vp env which corepack`

Core tool - corepack bundled with the resolved Node.js

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/js_runtime/node/<version>/bin/corepack
  Version:    20.18.0
  Source:     <workspace>/.node-version
```

## `vp install -g cowsay@1.6.0`

Install a global package via vp

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed cowsay 1.6.0
  Bins: cowsay, cowthink
```

## `vp env which cowsay`

Global package - shows binary path with metadata

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/packages/cowsay#<uuid>/lib/node_modules/cowsay/./cli.js
  Package:    cowsay@1.6.0
  Binaries:   cowsay, cowthink
  Node:       <version>
  Installed:  <date>
```

## `vp remove -g cowsay`

Cleanup

```
Uninstalled cowsay
```

## `vp env which unknown-tool`

Unknown tool - error message

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: tool 'unknown-tool' not found
Not a core tool (node, npm, npx, corepack) or installed global package.
Run 'vp list -g' to see installed packages.
```
