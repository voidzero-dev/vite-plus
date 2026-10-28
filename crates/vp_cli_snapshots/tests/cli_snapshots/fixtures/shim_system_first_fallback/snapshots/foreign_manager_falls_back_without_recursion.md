# foreign_manager_falls_back_without_recursion

## `node setup-foreign-manager.cjs`


## `vp env off node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node --version`

The external manager forwards to the managed fallback without re-entering itself.

```
<version>
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node node --version`

An inherited injection marker must not cause recursion either.

```
<version>
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} vp env exec node --version`

env exec still runs managed Node when the inherited PATH contains the external manager.

```
<version>
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node --version`

System-only Node.js detects the forwarding loop and fails instead of re-entering the external manager forever or using the managed runtime.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only requires a system 'node' on PATH that does not forward back to Vite+, but the one in '<workspace>/foreign-bin' re-entered the shim more than 8 times with the same arguments and directory
```

## `PATH=${workspace}/foreign-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm --version`

npm on system-only Node.js reports the loop that its query of the external manager ran into.

**Exit code:** 1

```
vp: Tool 'npm' not found: VP_NODE_MANAGER=system-only requires a system 'node' on PATH that does not forward back to Vite+, but the one in '<workspace>/foreign-bin' re-entered the shim more than 8 times with the same arguments and directory
```
