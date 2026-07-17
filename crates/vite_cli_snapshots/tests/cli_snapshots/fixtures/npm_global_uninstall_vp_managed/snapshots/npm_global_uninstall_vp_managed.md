# npm_global_uninstall_vp_managed

## `vp install -g ./npm-global-vp-managed-pkg`

Install via vp (creates managed shim)

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed npm-global-vp-managed-pkg 1.0.0
  Bins: npm-global-vp-managed-cli
```

## `npm install -g ./npm-global-vp-managed-pkg`

npm install (should warn about conflict)

```

added 1 package in <duration>
Skipped 'npm-global-vp-managed-cli': managed by `vp install -g npm-global-vp-managed-pkg`. Run `vp uninstall -g npm-global-vp-managed-pkg` to remove it first.
```

## `npm uninstall -g npm-global-vp-managed-pkg`

npm uninstall should NOT remove the vp-managed shim

```

removed 1 package in <duration>
```

## `vpt stat-file $VP_HOME/bin/npm-global-vp-managed-cli`

Shim should still exist

```
<home>/.vite-plus/bin/npm-global-vp-managed-cli: symlink
```

## `npm-global-vp-managed-cli`

Verify the shim still works

```
npm-global-vp-managed-cli works
```

## `vp remove -g npm-global-vp-managed-pkg`

Cleanup

```
Uninstalled npm-global-vp-managed-pkg
```
