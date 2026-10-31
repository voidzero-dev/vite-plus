# system_only_child_processes

System-only restrictions hold for what the selected tools run: a `#!/usr/bin/env node` shebang and a nested `npm` resolve from the child PATH, which loses the managed runtime directory whether a parent left it first on PATH or the shim would add it for managed Node.js. A nested managed npm would print its version instead of the system marker.

## `node setup-system-node.cjs`


## `node setup-child-tools.cjs`


## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node,npm,npx VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm nested`

System-only Node.js with npm in its own (managed) mode selects the npm beside system Node.js, but its `#!/usr/bin/env node` shebang reaches the system-only node shim first on the child PATH, which fails.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only requires a system 'node', but the Vite+ shim comes first on PATH; run `vp env off node` to move the shims after the system tools
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx node -p process.execPath.includes('system-bin')`

vpx looks up system-only Node.js on PATH without the inherited managed runtime.

```
true
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm nested`

vpx looks up system-only npm on PATH without the npm bundled with the inherited managed runtime. The nested npm reaches the system-only npm shim, which fails.

**Exit code:** 1

```
system npm on managed Node.js
vp: VP_NPM_MANAGER=system-only requires a system 'npm', but the Vite+ shim comes first on PATH; run `vp env off npm` to move the shims after the system tools
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm --version`

Without a system npm, vpx fails instead of running the inherited managed npm or downloading one through dlx.

**Exit code:** 1

```
error: vpx: VP_NPM_MANAGER=system-only requires a system 'npm' on PATH before the Vite+ shims, but none was found
```
