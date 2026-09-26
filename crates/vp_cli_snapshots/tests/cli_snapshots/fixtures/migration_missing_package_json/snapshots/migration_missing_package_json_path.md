# migration_missing_package_json_path

## `vpt mkdir empty`


## `vp migrate empty --no-interactive --no-hooks --no-agent`

An explicit path without package.json gets the same error

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Cannot migrate empty: no package.json found. Run vp migrate from a project root or pass its path explicitly.
```
