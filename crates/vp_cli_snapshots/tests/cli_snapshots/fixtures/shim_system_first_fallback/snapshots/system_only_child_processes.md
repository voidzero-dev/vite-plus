# system_only_child_processes

System-only restrictions hold for what the selected tools run: a `#!/usr/bin/env node` shebang and a nested `npm` resolve from the child PATH, which loses the managed runtime directory whether a parent left it first on PATH or the shim would add it for managed Node.js. A nested managed npm would print its version instead of the system marker.

## `node setup-system-node.cjs`


## `node setup-child-tools.cjs`


## `PATH=${VP_HOME}/js_runtime/node/20.18.0/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node,npm,npx VP_NODE_MANAGER=system-only VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm nested`

With the managed runtime inherited first on PATH, system npm runs on system Node.js, and so does its nested npm.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/js_runtime/node/20.18.0/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm nested`

System-only npm on an inherited managed Node.js: Node.js stays managed, and the nested npm is the system one, not the npm bundled with the runtime.

```
system npm on managed Node.js
system npm child on managed Node.js
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm nested`

System-only npm on the pinned managed Node.js: the shim does not expose the runtime directory, so the nested npm is the system one.

```
system npm on managed Node.js
system npm child on managed Node.js
```

## `PATH=${VP_HOME}/js_runtime/node/20.18.0/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node,npm,npx VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm nested`

System-only Node.js with npm in its own (managed) mode: the npm beside system Node.js runs on it, and the nested npm does not reach the npm bundled with the inherited managed runtime.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm nested`

Both system-only without an inherited managed runtime: system npm and its nested npm run on system Node.js.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/js_runtime/node/20.18.0/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx node -p process.execPath.includes('system-bin')`

vpx looks up system-only Node.js on PATH without the inherited managed runtime.

```
true
```

## `PATH=${VP_HOME}/js_runtime/node/20.18.0/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm nested`

vpx looks up system-only npm on PATH without the npm bundled with the inherited managed runtime.

```
system npm on managed Node.js
system npm child on managed Node.js
```

## `PATH=${VP_HOME}/js_runtime/node/20.18.0/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} EXPECTED_PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs node -p 'process.env.PATH === process.env.EXPECTED_PATH'`

System-only Node.js removes the inherited managed runtime directory from the child PATH and keeps the other entries in order.

```
true
```

## `PATH=${VP_HOME}/js_runtime/node/20.18.0/bin${PATH_SEPARATOR}${VP_HOME}/bin VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm --version`

Without a system npm, vpx fails instead of running the inherited managed npm or downloading one through dlx.

**Exit code:** 1

```
error: vpx: VP_NPM_MANAGER=system-only requires a system 'npm' on PATH, but none was found
```
