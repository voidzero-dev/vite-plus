# npm8_global_config

## `vpt write-file package.json {}`


## `vpt write-file .npmrc 'tag=next
'`


## `vp install -g testnpm2@^1`


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

A user-selected tag changes wanted while latest stays on the registry's latest tag

**Exit code:** 1

```
testnpm2 (global)
1.0.0 => 1.0.1 (latest: 1.0.0)
```
