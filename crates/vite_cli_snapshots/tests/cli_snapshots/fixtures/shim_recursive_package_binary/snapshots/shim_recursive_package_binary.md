# shim_recursive_package_binary

## `vp install -g ./recursive-cli-pkg`

Install test package

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed recursive-cli-pkg 1.0.0
  Bins: recursive-cli
```

## `recursive-cli`

Outer call triggers recursive inner call through shim

```
outer call
inner call succeeded
```

## `vp remove -g recursive-cli-pkg`

Cleanup

```
Uninstalled recursive-cli-pkg
```
