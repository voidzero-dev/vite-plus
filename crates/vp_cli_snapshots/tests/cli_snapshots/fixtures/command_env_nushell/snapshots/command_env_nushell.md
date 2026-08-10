# command_env_nushell

## `VP_HOME=${workspace}/vp "home\with spaces" vp env setup --refresh`


## `vpt cp assert.nu 'vp "home\with spaces"/assert.nu'`


## `cd 'vp "home\with spaces"' && EXPECTED_VP_HOME=${workspace} PATH=${workspace}/bin:${workspace}/bin:${PATH} nu assert.nu`

loads the generated env.nu and verifies the Nushell wrapper

```
Using Node.js <version> (resolved from 20.18.0)
Reverted to file-based Node.js version resolution
Nushell environment checks passed
```
