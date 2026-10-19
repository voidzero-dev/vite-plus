# command_env_which

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

## `vp install -g vp-test-echo@1.0.0`

Install a global package via vp

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
 Installed vp-test-echo 1.0.0
  Bins: vp-test-echo, vp-test-echo-alias
```

## `vp env which vp-test-echo`

Global package - shows binary path with metadata

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/packages/vp-test-echo/<uuid>/lib/node_modules/vp-test-echo/cli.cjs
  Package:    vp-test-echo@1.0.0
  Binaries:   vp-test-echo, vp-test-echo-alias
  Node:       <version>
  Installed:  <date>
```

## `vp remove -g vp-test-echo`

Cleanup

```
- Uninstalled vp-test-echo
```

## `vp env which unknown-tool`

Unknown tool - error message

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: tool 'unknown-tool' not found
Not a core tool (node, npm, npx) or installed global package.
Run 'vp list -g' to see installed packages.
```
