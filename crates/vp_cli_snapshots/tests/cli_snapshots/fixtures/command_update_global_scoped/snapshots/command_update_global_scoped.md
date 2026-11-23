# command_update_global_scoped

## `vp install -g testnpm2@1.0.0`


## `vpt json-edit $VP_HOME/packages/testnpm2.json versionSpec null`

make testnpm2 outdated so an unscoped lookup would update it


## `vp update -g ./local-tool`

should update only the local package

```
info: Updating 1 global package with Node.js <version>
 Updated vp-snap-local-tool to 0.0.0
  Bins: vp-snap-local-tool
```

## `vp list -g testnpm2`

should keep testnpm2 at 1.0.0

```
Package          Node version   Binaries
---              ---            ---
testnpm2@1.0.0   <version>
```
