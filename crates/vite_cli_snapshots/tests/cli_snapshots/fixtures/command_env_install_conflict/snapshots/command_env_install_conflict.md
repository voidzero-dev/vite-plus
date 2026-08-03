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
