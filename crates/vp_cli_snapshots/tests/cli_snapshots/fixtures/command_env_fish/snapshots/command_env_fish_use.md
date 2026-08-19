# command_env_fish_use

## `VP_HOME=${workspace}/vp "$project\with spaces" vp env setup --refresh`


## `vpt cp assert_use.fish 'vp "$project\with spaces"/assert_use.fish'`


## `vpt write-file 'vp "$project\with spaces"/.node-version' '22.18.0
'`


## `cd 'vp "$project\with spaces"' && PATH=${workspace}/bin:${PATH} fish --no-config assert_use.fish`

verifies the Fish wrapper help, explicit use, unset, and file-based use branches

```
Using Node.js <version> (resolved from 20.18.0)
Reverted to file-based Node.js version resolution
Using Node.js <version> (resolved from .node-version)
Fish environment use checks passed
```
