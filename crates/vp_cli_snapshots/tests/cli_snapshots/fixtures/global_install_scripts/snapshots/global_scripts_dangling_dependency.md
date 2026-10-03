# global_scripts_dangling_dependency

A dangling dependency link in a local package must not prevent global installation or skipped-script warnings.

## `vpt mkdir -p local-cli/node_modules`


## `node -e 'require('\''node:fs'\'').symlinkSync(require('\''node:path'\'').resolve('\''removed-package'\''), '\''local-cli/node_modules/unused-link'\'', process.platform === '\''win32'\'' ? '\''junction'\'' : '\''dir'\'')'`


## `vp install -g ./local-cli`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed local-cli 1.0.0
  Bins: local-cli
warning: Lifecycle scripts were skipped for: local-cli.
To allow them, reinstall with:
  vp install -g ./local-cli --run-scripts
```

## `local-cli`

```
local-cli works
```

## `vpt stat-file local-cli/postinstall-ran --assert missing`

```
local-cli/postinstall-ran: missing
```
