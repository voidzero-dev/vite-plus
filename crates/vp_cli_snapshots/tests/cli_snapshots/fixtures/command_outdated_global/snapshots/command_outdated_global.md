# command_outdated_global

## `vp install -g testnpm2@1.0.0`

should prepare a version-pinned global package


## `vp outdated definitely-not-installed-vite-plus-snap-pkg -g --format json`

should support empty global json output

```
{}
```

## `vp outdated testnpm2 -g --format json`

should report a pinned package without a wanted update

**Exit code:** 1

```
{
  "testnpm2": {
    "current": "1.0.0",
    "wanted": "1.0.0",
    "latest": "1.0.1",
    "dependent": "global",
    "location": "<home>/.vite-plus/packages/testnpm2/<uuid>/lib/node_modules/testnpm2"
  }
}
```

## `vp outdated testnpm2 -g --format list`

should render the newer latest as a hint in list format

**Exit code:** 1

```
testnpm2 (global)
1.0.0 => 1.0.0 (latest: 1.0.1)
```

## `vp update -g`

should not move a pinned package to latest

```
All global packages are up to date.
```

## `vpt json-edit $VP_HOME/packages/testnpm2.json versionSpec no-such-tag`

should warn and skip when the recorded version spec no longer resolves


## `vp update -g`

**Exit code:** 1

```
All global packages are up to date.
[1m[33mwarn:[39m[0m npm view failed for testnpm2@no-such-tag: npm error code E404; skipping
```

## `vpt json-edit $VP_HOME/packages/testnpm2.json versionSpec null`

should follow latest again once the recorded version spec is cleared


## `vp outdated testnpm2 -g --format json`

should support global json output

**Exit code:** 1

```
{
  "testnpm2": {
    "current": "1.0.0",
    "wanted": "1.0.1",
    "latest": "1.0.1",
    "dependent": "global",
    "location": "<home>/.vite-plus/packages/testnpm2/<uuid>/lib/node_modules/testnpm2"
  }
}
```

## `vp outdated testnpm2 -g --format list --concurrency 5`

should support global list output

**Exit code:** 1

```
testnpm2 (global)
1.0.0 => 1.0.1
```

## `vpt json-edit $VP_HOME/packages/testnpm2.json versionSpec 1.0.0`

should override a recorded version spec with --latest


## `vp update -g --latest`

```
[1m[94minfo:[39m[0m Updating 1 global package with Node.js <version>
[32m✓[39m Updated [1mtestnpm2[0m to [1m1.0.1[0m
```

## `vpt grep-file $VP_HOME/packages/testnpm2.json versionSpec`

should clear the recorded version spec after --latest (grep-file prints missing)

**Exit code:** 1

```
<home>/.vite-plus/packages/testnpm2.json: missing "versionSpec"
pattern not found
```

## `vpt json-edit $VP_HOME/packages/testnpm2.json versionSpec 1.0.1`

should clear a recorded version spec with --latest even without a reinstall


## `vp update -g --latest`

```
All global packages are up to date.
```

## `vpt grep-file $VP_HOME/packages/testnpm2.json versionSpec`

should have removed the pin from the up-to-date package (grep-file prints missing)

**Exit code:** 1

```
<home>/.vite-plus/packages/testnpm2.json: missing "versionSpec"
pattern not found
```

## `vpt json-edit $VP_HOME/packages/testnpm2.json versionSpec 1.0.0`

should persist an explicit spec switch without a reinstall


## `vp update -g testnpm2@1.0.1`

```
All global packages are up to date.
```

## `vpt grep-file $VP_HOME/packages/testnpm2.json 'versionSpec": "1.0.1'`

```
<home>/.vite-plus/packages/testnpm2.json: found "versionSpec\": \"1.0.1"
```

## `vp update -g testnpm2@no-such-tag`

should not persist an explicit spec that fails to resolve

**Exit code:** 1

```
All global packages are up to date.
[1m[33mwarn:[39m[0m npm view failed for testnpm2@no-such-tag: npm error code E404; skipping
```

## `vpt grep-file $VP_HOME/packages/testnpm2.json 'versionSpec": "1.0.1'`

```
<home>/.vite-plus/packages/testnpm2.json: found "versionSpec\": \"1.0.1"
```
