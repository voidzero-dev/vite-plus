# command_env_doctor_configuration

## `vp env doctor`

configuration modes read naturally without extending their labels

```
VITE+ - The Unified Toolchain for the Web

Installation
  ✓ VP_HOME           ~/.vite-plus
  ✓ Bin directory     exists
  ✓ Shims             node, npm, npx, pnpm, pnpx, yarn, yarnpkg, bun, bunx, vpx, vpr

Configuration
  ✓ Node.js           managed mode
  ✓ Package manager   managed mode

PATH
  ✓ vp                in PATH
  ✓ node              ~/.vite-plus/bin/node (vp shim)
  ✓ npm               ~/.vite-plus/bin/npm (vp shim)
  ✓ npx               ~/.vite-plus/bin/npx (vp shim)
  ✓ pnpm              ~/.vite-plus/bin/pnpm (vp shim)
  ✓ pnpx              ~/.vite-plus/bin/pnpx (vp shim)
  ✓ yarn              ~/.vite-plus/bin/yarn (vp shim)
  ✓ yarnpkg           ~/.vite-plus/bin/yarnpkg (vp shim)
  ✓ bun               ~/.vite-plus/bin/bun (vp shim)
  ✓ bunx              ~/.vite-plus/bin/bunx (vp shim)
  ✓ vpx               ~/.vite-plus/bin/vpx (vp shim)
  ✓ vpr               ~/.vite-plus/bin/vpr (vp shim)

Node.js Resolution
  Directory         <workspace>
  Source            <workspace>/.node-version
  Version           20.18.0
  ⚠ Node binary       not installed
  note: Version will be downloaded on first use.

Package Manager Resolution
  Package manager   not selected

IDE Setup
  ⚠ GUI applications may not see shell PATH changes.

  macOS:
  Add to ~/.zshenv or ~/.profile:
  . "$HOME/.vite-plus/env"
  Then restart your IDE to apply changes.

✓ All checks passed
```
