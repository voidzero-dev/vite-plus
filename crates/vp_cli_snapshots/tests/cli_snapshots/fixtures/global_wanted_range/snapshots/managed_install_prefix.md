# managed_install_prefix

## `vpt write-file other-prefix/etc/npmrc 'tag=next
'`


## `npm_config_prefix=${workspace}/other-prefix vp install -g testnpm2`


## `npm_config_prefix=${workspace}/other-prefix vp outdated -g --format json`

```
{}
```

## `npm_config_prefix=${workspace}/other-prefix vp update -g`

```
All global packages are up to date.
```
