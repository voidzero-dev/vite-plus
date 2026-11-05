# vpx_script_follows_vp_node

vpx script mode runs on the Node.js that vp node selects, including system-first mode and VP_ENV_FORBIDDEN=1.

## `node ../setup-system-node.cjs`


## `vp env on`


## `vp env off node`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vpx --print process.execPath.includes('system-bin')`

System-first: the system Node.js found on PATH, as vp node selects.

```
true
```

## `PATH=${VP_HOME}/fallback-bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vpx --print process.execPath.includes('system-bin')`

A Vite+ shim before system Node.js selects managed Node.js.

```
false
```

## `vp env on node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vpx --print process.execPath.includes('system-bin')`

Managed mode ignores system Node.js.

```
false
```

## `PATH=${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${PATH} VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx --print process.execPath.includes('system-bin')`

VP_ENV_FORBIDDEN=1: the system Node.js first on PATH, as vp node selects, even with managed config.json.

```
true
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx --print process.execPath.includes('system-bin')`

VP_ENV_FORBIDDEN=1 with the node shim first on PATH fails and suggests `vp env off node`, as vp node does.

**Exit code:** 1

```
error: vpx: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'node', but the Vite+ shim comes first on PATH; run `vp env off node` to move the shims after the system tools
```
