# command_env_nushell_shadowed_builtins

## `VP_HOME=${workspace}/home vp env setup --refresh`


## `vpt cp assert_shadowed.nu home/assert_shadowed.nu`


## `cd home && VP_HOME=${workspace}/home nu assert_shadowed.nu`

env.nu loads and runs `vp env use` when user commands shadow the built-ins it calls (#3092)

```
Using Node.js <version> (resolved from 20.20.2)
Reverted selected components to project environment resolution
env.nu loads and works with shadowed built-ins
```
