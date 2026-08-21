# env_use_warns_when_package_manager_differs

## `vp env use yarn@4.12.0 --no-install`

an explicit project manager warns before a different session manager is used

```
warn: Current environment resolves to pnpm from packageManager, but yarn was requested.
export VP_PACKAGE_MANAGER=yarn@4.12.0
Using yarn <version> (resolved from 4.12.0)
```
