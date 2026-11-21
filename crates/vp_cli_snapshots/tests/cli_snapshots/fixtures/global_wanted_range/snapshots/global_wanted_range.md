# global_wanted_range

## `vp install -g testnpm2@^1 range-alias@npm:testnpm2@^1`


## `vp list -g`

npm prefers the matching latest tag for both installs

```
Package                          Node version   Binaries
---                              ---            ---
range-alias@npm:testnpm2@1.0.0   <version>
testnpm2@1.0.0                   <version>
```

## `vp outdated -g --format json`

```
{}
```

## `vp update -g`

both installed versions already satisfy the update

```
All global packages are up to date.
```
