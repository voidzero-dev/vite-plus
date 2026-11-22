# global_wanted_range

## `vp install -g testnpm2@^01 range-alias@npm:testnpm2@^1`


## `vp outdated -g testnpm2 range-alias --format list`

The highest matching version is newer than the latest tag for both ranges

**Exit code:** 1

```
testnpm2 (global)
1.0.0 => 1.0.1 (latest: 1.0.0)

range-alias (global)
testnpm2@1.0.0 => testnpm2@1.0.1 (latest: testnpm2@1.0.0)
```

## `vp outdated -g testnpm2@* --format list`

An explicit wildcard includes all matching releases

**Exit code:** 1

```
testnpm2 (global)
1.0.0 => 1.0.1 (latest: 1.0.0)
```

## `vp update -g testnpm2 range-alias`

```
info: Updating 2 global packages with Node.js <version>
 Updated testnpm2 to 1.0.1

 Updated range-alias to 1.0.1
```

## `vp list -g`

```
Package                          Node version   Binaries
---                              ---            ---
range-alias@npm:testnpm2@1.0.1   <version>
testnpm2@1.0.1                   <version>
```

## `vpt grep-file $VP_HOME/packages/testnpm2.json 'versionSpec": "^01"'`

```
<home>/.vite-plus/packages/testnpm2.json: found "versionSpec\": \"^01\""
```

## `vpt grep-file $VP_HOME/packages/range-alias.json 'versionSpec": "npm:testnpm2@^1"'`

```
<home>/.vite-plus/packages/range-alias.json: found "versionSpec\": \"npm:testnpm2@^1\""
```

## `vp update -g`

```
All global packages are up to date.
```

## `VP_NODE_VERSION=18.12.1 vp update -g testnpm2 --reinstall-node-mismatch`

Reinstalling for another Node version keeps the resolved version

```
info: Updating 1 global package with Node.js <version>
 Updated testnpm2 to 1.0.1
```
