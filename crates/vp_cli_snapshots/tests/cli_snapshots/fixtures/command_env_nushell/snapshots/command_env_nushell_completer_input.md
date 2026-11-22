# command_env_nushell_completer_input

## `VP_HOME=${workspace}/home vp env setup --refresh`


## `vpt cp assert_completion.nu home/assert_completion.nu`


## `cd home && VP_HOME=${workspace}/home nu assert_completion.nu`

env.nu completers complete `vp` after a pipeline and do not use the positional input that Nushell 0.116 deprecates (#3127)

```
Nushell completer checks passed
```
