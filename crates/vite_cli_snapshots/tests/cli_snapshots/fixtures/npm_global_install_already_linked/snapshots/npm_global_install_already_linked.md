# npm_global_install_already_linked

## `npm config get prefix`

```
<npm-prefix>
```

## `vp install -g ./npm-global-linked-pkg`

First install via vp (creates managed shim)

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed npm-global-linked-pkg 1.0.0
  Bins: npm-global-linked-cli
```

## `npm-global-linked-cli`

Should be callable via the link

```
npm-global-linked-cli works
```

## `npm install -g ./npm-global-linked-pkg`

Should not create duplicate link

```

added 1 package in <duration>
Skipped 'npm-global-linked-cli': managed by `vp install -g npm-global-linked-pkg`. Run `vp uninstall -g npm-global-linked-pkg` to remove it first.
```

## `vp remove -g npm-global-linked-pkg`

Cleanup

```
Uninstalled npm-global-linked-pkg
```

## `vpt stat-file $VP_HOME/bin/npm-global-linked-cli`

link should be removed

```
<home>/.vite-plus/bin/npm-global-linked-cli: missing
```
