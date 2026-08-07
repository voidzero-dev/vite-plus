# command_env_setup_external_vp

## `vpt mkdir -p external/current/bin external/bin external/js_runtime/node/22.18.0/bin`

A second, complete legacy install outside the case home


## `vpt cp $VP_HOME/current/bin/vp external/current/bin/vp`

The external install's vp binary


## `vpt cp $VP_HOME/current/bin/vp external/bin/vp`

Marks the external layout as a legacy install for detection


## `vpt chmod +x external/current/bin/vp`


## `vpt chmod +x external/bin/vp`


## `vpt write-file .node-version '22.18.0
'`

Project Node.js version


## `vpt write-file external/js_runtime/node/22.18.0/bin/node '#'\!'/bin/sh
echo vp-managed-node-22.18.0
'`

Preinstall managed Node runtime


## `vpt chmod +x external/js_runtime/node/22.18.0/bin/node`


## `VP_HOME=${workspace}/external ./external/current/bin/vp env setup`

env setup targets the external install via VP_HOME


## `node assert-shims.mjs`

Shims point to the external install's vp, not the case home's

```
all shims point to the external install
```

## `VP_HOME=${workspace}/external PATH=${workspace}/external/bin:${PATH} node -v`

node shim uses the project version

```
vp-managed-node-22.18.0
```
