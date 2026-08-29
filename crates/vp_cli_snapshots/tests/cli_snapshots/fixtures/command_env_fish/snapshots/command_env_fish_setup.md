# command_env_fish_setup

## `VP_HOME=${workspace}/vp "$project\with spaces" vp env setup --refresh`


## `vpt cp assert_setup.fish 'vp "$project\with spaces"/assert_setup.fish'`


## `cd 'vp "$project\with spaces"' && EXPECTED_VP_HOME=${workspace} PATH=${workspace}/bin:${workspace}/bin:${PATH} fish --no-config assert_setup.fish`

loads the generated env.fish and verifies Fish path setup and command passthrough

```
Fish environment setup checks passed
```
