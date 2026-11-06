# command_env_nushell_without_fish

## `VP_HOME=${workspace}/home vp env setup --refresh`


## `vpt cp assert_without_fish.nu home/assert_without_fish.nu`


## `cd home && VP_HOME=${workspace}/home nu assert_without_fish.nu`

without fish on PATH, env.nu completers return null so Nushell 0.116 falls back to file completion (#3272)

```
Nushell completer checks without fish passed
```
