# command_env_fish_use

## `VP_HOME=${workspace}/vp "$project\with spaces" vp env setup --refresh`


## `vpt cp assert_use.fish 'vp "$project\with spaces"/assert_use.fish'`


## `vpt write-file 'vp "$project\with spaces"/.node-version' 22.18.0`


## `cd 'vp "$project\with spaces"' && PATH=${workspace}/bin:${PATH} fish --no-config assert_use.fish`

verifies the Fish wrapper help, explicit use, unset, file-based use, and failure branches

```
Using Node.js <version> (resolved from 20.18.0)
Reverted selected components to project environment resolution
Using Node.js <version> (resolved from .node-version)
error: Unexpected argument '--invalid-option'

Use `-- --invalid-option` to pass the argument as a value
Fish environment use checks passed
```
