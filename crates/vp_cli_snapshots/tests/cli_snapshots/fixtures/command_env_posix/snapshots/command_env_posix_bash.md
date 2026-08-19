# command_env_posix_bash

## `VP_HOME=${workspace}/vp "$project\with `spaces`" vp env setup --refresh`


## `vpt cp assert_posix.sh 'vp "$project\with `spaces`"/assert_posix.sh'`


## `vpt write-file 'vp "$project\with `spaces`"/.node-version' '22.18.0
'`


## `cd 'vp "$project\with `spaces`"' && EXPECTED_VP_HOME=${workspace} SHELL_LABEL=bash PATH=${workspace}/bin:${workspace}/bin:${PATH} bash --noprofile --norc assert_posix.sh`

loads the generated env file in Bash and verifies PATH, wrapper, completions, and version switching

```
Using Node.js <version> (resolved from 20.18.0)
Reverted to file-based Node.js version resolution
Using Node.js <version> (resolved from .node-version)
POSIX environment checks passed (bash)
```
