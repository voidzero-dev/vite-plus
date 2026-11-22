# bundled_npm_through_system_node_shim

## `vpt write-file .node-version 20.20.2`


## `vp env exec --node 22.23.4 node assert-system-node-shim.cjs setup`


## `vp env off node`


## `vp env on npm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-shims${PATH_SEPARATOR}/usr/bin${PATH_SEPARATOR}/bin ./system-shims/node assert-system-node-shim.cjs`

Launch the external Node shim directly so npm/npx resolve through vp without inheriting VP_BYPASS

```
Bundled npm/npx use the runtime behind the system Node shim
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-shims${PATH_SEPARATOR}/usr/bin${PATH_SEPARATOR}/bin ./system-shims/node assert-system-node-shim.cjs preload`

Preload output cannot corrupt the runtime probe, and npm/npx and their Node children still execute the preload

```
Bundled npm/npx use the runtime behind the system Node shim
```
