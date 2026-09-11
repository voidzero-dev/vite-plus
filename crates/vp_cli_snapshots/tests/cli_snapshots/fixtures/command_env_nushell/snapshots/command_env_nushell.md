# command_env_nushell

## `VP_HOME=${workspace}/vp "home\with spaces" vp env setup --refresh`


## `vpt cp assert.nu 'vp "home\with spaces"/assert.nu'`


## `vpt write-file 'vp "home\with spaces"/.node-version' '22.18.0
'`


## `cd 'vp "home\with spaces"' && EXPECTED_VP_HOME=${workspace} PATH=${workspace}/bin:${workspace}/bin:${PATH} nu assert.nu`

loads the generated env.nu and verifies setup, explicit use, unset, and file-based use

```
Using Node.js <version> (resolved from 20.18.0)
Reverted selected components to project environment resolution
Using Node.js <version> (resolved from .node-version)
Nushell environment checks passed
```
