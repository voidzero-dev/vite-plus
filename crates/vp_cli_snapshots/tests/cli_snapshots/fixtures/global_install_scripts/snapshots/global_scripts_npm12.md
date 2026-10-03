# global_scripts_npm12

A case-owned Node runtime with bundled npm 12 still runs all scripts only when explicitly enabled, without changing Node.js.

## `node setup-npm.cjs`


## `node -p 'require(require('\''node:path'\'').join(require('\''node:path'\'').dirname(process.execPath), process.platform === '\''win32'\'' ? '\''node_modules/npm/package.json'\'' : '\''../lib/node_modules/npm/package.json'\'')).version'`

```
12.0.2
```

## `vpt mkdir -p tarballs`


## `npm pack ./native-addon --ignore-scripts --pack-destination tarballs`


## `npm pack ./my-cli --ignore-scripts --pack-destination tarballs`


## `vp install -g my-cli@1.0.0`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed my-cli 1.0.0
  Bins: my-cli
warning: Lifecycle scripts were skipped for: my-cli, native-addon.
To allow them, reinstall with:
  vp install -g my-cli@1.0.0 --run-scripts
```

## `my-cli skipped skipped`

```
my-cli: skipped; native-addon: skipped
```

## `vp install -g my-cli@1.0.0 --run-scripts`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed my-cli 1.0.0
  Bins: my-cli
```

## `my-cli ran ran`

```
my-cli: ran; native-addon: ran
```
