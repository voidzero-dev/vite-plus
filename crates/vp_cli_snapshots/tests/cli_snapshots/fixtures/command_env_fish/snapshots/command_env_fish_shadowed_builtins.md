# command_env_fish_shadowed_builtins

## `VP_HOME=${workspace}/home vp env setup --refresh`


## `vpt cp assert_shadowed.fish home/assert_shadowed.fish`


## `vpt write-file home/package.json '{ "name": "demo", "scripts": { "build": "echo build" } }'`


## `cd home && VP_HOME=${workspace}/home fish --no-config assert_shadowed.fish`

env.fish loads and runs `vp env use` and the vpr completion when user functions shadow the built-ins it calls (#3128)

```
Using Node.js <version> (resolved from 20.20.2)
Using Node.js <version> (resolved from 22.23.4)
env.fish loads and works with shadowed built-ins
```
