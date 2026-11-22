# npm8_global_config

## `vpt write-file package.json {}`


## `vpt write-file .npmrc 'tag=next
'`


## `vp install -g testnpm2`


## `vp list -g`

npm 8 ignores the project's default tag for global installs

```
Package          Node version   Binaries
---              ---            ---
testnpm2@1.0.0   <version>
```

## `vp outdated -g --format json`

```
{}
```

## `vp update -g`

```
All global packages are up to date.
```

## `npm_config_tag=next vp outdated -g --format list`

The user can select a default tag for a bare spec

**Exit code:** 1

```
testnpm2 (global)
1.0.0 => 1.0.1 (latest: 1.0.0)
```

## `vp update -g testnpm2@next`

```
info: Updating 1 global package with Node.js <version>
 Updated testnpm2 to 1.0.1
```

## `vpt grep-file $VP_HOME/packages/testnpm2.json 'versionSpec": "next"'`

```
<home>/.vite-plus/packages/testnpm2.json: found "versionSpec\": \"next\""
```

## `npm_config_tag=next vp update -g --latest`

An explicit --latest follows the registry's latest tag

```
info: Updating 1 global package with Node.js <version>
 Updated testnpm2 to 1.0.0
```
