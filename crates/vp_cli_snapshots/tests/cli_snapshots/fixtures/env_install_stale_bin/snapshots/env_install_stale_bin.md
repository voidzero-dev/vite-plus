# env_install_stale_bin

## `vp install -g ./env-install-stale-bin-pkg-v1`

Install package with two binaries

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed env-install-stale-bin-pkg 1.0.0
  Bins: env-install-stale-drop, env-install-stale-keep
```

## `env-install-stale-keep`

Both binaries should be callable

```
env-install-stale-keep ok
```

## `env-install-stale-drop`

```
env-install-stale-drop ok
```

## `vp install -g ./env-install-stale-bin-pkg-v2`

Reinstall package version that removed one binary

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed env-install-stale-bin-pkg 2.0.0
  Bins: env-install-stale-keep
```

## `env-install-stale-keep`

Remaining binary should still be callable

```
env-install-stale-keep ok
```

## `node check-stale-binary.js`

```
stale shim removed
stale config removed
```
