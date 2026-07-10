# command_env_install_conflict

## `vp install -g ./conflict-pkg`

Install package with conflicting binary name (uses cwd version)

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
warn: Package 'conflict-pkg' provides 'node' binary, but it conflicts with a built-in shim. Skipping.
✓ Installed conflict-pkg 1.0.0
  Bins: conflict-cli
```

## `vp remove -g conflict-pkg`

Cleanup

```
Uninstalled conflict-pkg
```

## `vp install -g --node 20 ./conflict-pkg`

Install with specific Node.js version

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
error: Failed to install conflict-pkg: Package was not installed correctly, package.json not found at <home>/.vite-plus/packages/conflict-pkg#<uuid>/lib/node_modules/conflict-pkg/package.json
```

## `vp remove -g conflict-pkg`

Cleanup

**Exit code:** 1

```
Failed to uninstall conflict-pkg: Package conflict-pkg is not installed
```
