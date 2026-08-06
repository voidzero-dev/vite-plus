# command_env_setup_external_vp

## `vpt mkdir -p external home`

Prepare isolated external install and VP_HOME


## `vpt cp $VP_HOME/bin/vp external/vp`

Simulate a Homebrew-style vp outside VP_HOME


## `vpt chmod +x external/vp`


## `vpt write-file .node-version '22.18.0
'`

Project Node.js version


## `vpt write-file home/js_runtime/node/22.18.0/bin/node '#'\!'/bin/sh
echo vp-managed-node-22.18.0
'`

Preinstall managed Node runtime


## `vpt chmod +x home/js_runtime/node/22.18.0/bin/node`


## `VP_HOME=${workspace}/home ./external/vp env setup`

Setup shims from external vp


## `node assert-shims.mjs`

Shims should point to external vp, not VP_HOME/current/bin/vp

```
all shims point to external vp
```

## `VP_HOME=${workspace}/home PATH=${workspace}/home/bin:${PATH} node -v`

node shim uses the project version

```
vp-managed-node-22.18.0
```
