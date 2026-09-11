# global_install_dynamic_import

Regression test for #2220: globally installed packages can dynamically import an absolute path inside their installation.

## `vp install -g .`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed global-install-dynamic-import 0.0.0
  Bins: global-install-dynamic-import
```

## `global-install-dynamic-import`

```
global dynamic import loaded
```
