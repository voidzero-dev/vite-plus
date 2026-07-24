# command_env_install_global_local

## `vp install -g .`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed just-a-normal-package 0.0.0
  Bins: just-a-normal-package
```

## `vp install -g ./another-package.tgz`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed another-normal-package 0.0.1
  Bins: another-normal-package
```

## `vp list -g just-a-normal-package`

```
Package                       Node version   Binaries
---                           ---            ---
just-a-normal-package@0.0.0   24.18.0        just-a-normal-package
```

## `vp list -g another-normal-package`

```
Package                        Node version   Binaries
---                            ---            ---
another-normal-package@0.0.1   24.18.0        another-normal-package
```
