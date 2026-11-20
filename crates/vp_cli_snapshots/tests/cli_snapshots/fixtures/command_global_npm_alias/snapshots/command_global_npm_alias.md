# command_global_npm_alias

## `vp install -g testnpm2-alias@npm:testnpm2@1.0.0`

should prepare a version-pinned npm alias


## `vp list -g`

should list the alias with its aliased package

```
Package                             Node version   Binaries
---                                 ---            ---
testnpm2-alias@npm:testnpm2@1.0.0   <version>
```

## `vp outdated testnpm2-alias -g --format json`

should resolve versions of the aliased package

**Exit code:** 1

```
{
  "testnpm2-alias": {
    "current": "testnpm2@1.0.0",
    "wanted": "testnpm2@1.0.0",
    "latest": "testnpm2@1.0.1",
    "dependent": "global",
    "location": "<home>/.vite-plus/packages/testnpm2-alias/<uuid>/lib/node_modules/testnpm2-alias"
  }
}
```

## `vp update -g`

should keep a pinned alias in place

```
All global packages are up to date.
```

## `vp update -g testnpm2-alias --latest`

should update the aliased package instead of a package named after the alias

```
info: Updating 1 global package with Node.js <version>
 Updated testnpm2-alias to 1.0.1
```

## `vpt grep-file $VP_HOME/packages/testnpm2-alias.json 'versionSpec": "npm:testnpm2"'`

should keep the alias after --latest drops the pin

```
<home>/.vite-plus/packages/testnpm2-alias.json: found "versionSpec\": \"npm:testnpm2\""
```

## `vp list -g`

```
Package                             Node version   Binaries
---                                 ---            ---
testnpm2-alias@npm:testnpm2@1.0.1   <version>
```
