# env_pin_warns_when_package_manager_differs

## `vp env pin yarn@4.12.0 --no-install --force`

an explicit project manager warns before a different manager is pinned

```
VITE+ - The Unified Toolchain for the Web

warn: Current environment resolves to pnpm from packageManager, but yarn was requested.
✓ Pinned package manager to yarn@4.12.0
note: Package manager will be downloaded on first use.
```
