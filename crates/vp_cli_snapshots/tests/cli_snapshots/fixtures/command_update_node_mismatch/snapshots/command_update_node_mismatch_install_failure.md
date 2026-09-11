# command_update_node_mismatch_install_failure

## `vp install -g --node 20 semver@7.7.2`


## `vpt json-edit $VP_HOME/bins/semver.json package conflicting-package`


## `vp update -g semver@=7.7.2 --reinstall-node-mismatch`

should keep the recorded spec when the reinstall fails

**Exit code:** 1

```
[1m[94minfo:[39m[0m Updating 1 global package with Node.js <version>
[1m[31merror:[39m[0m Failed to update semver: Executable 'semver' is already installed by conflicting-package

Please remove conflicting-package before installing semver, or use --force to auto-replace
```

## `vpt grep-file $VP_HOME/packages/semver.json 'versionSpec": "7.7.2'`

should keep metadata from the successful install

```
<home>/.vite-plus/packages/semver.json: found "versionSpec\": \"7.7.2"
```
