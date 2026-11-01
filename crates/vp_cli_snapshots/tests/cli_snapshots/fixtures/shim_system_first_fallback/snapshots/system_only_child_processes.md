# system_only_child_processes

System-only restrictions hold for what the selected tools run: a `#!/usr/bin/env node` shebang and a nested `npm` resolve from the child PATH, which loses the managed runtime directory whether a parent left it first on PATH or the shim would add it for managed Node.js. A nested managed npm would print its version instead of the system marker.

## `node setup-system-node.cjs`


## `node setup-child-tools.cjs`


## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node,npm,npx VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm nested`

System-only Node.js with npm in its own (managed) mode: the npm shim finds the inherited managed runtime as the first Node.js on PATH, and fails with a pointer to `vp env doctor` instead of running it.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only rejects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/node' that PATH selects for 'node'; run `vp env doctor` to check PATH
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx node -p process.execPath.includes('system-bin')`

vpx takes system-only Node.js from the first PATH match, the inherited managed runtime, and fails instead of skipping it.

**Exit code:** 1

```
error: vpx: VP_NODE_MANAGER=system-only rejects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/node' that PATH selects for 'node'; run `vp env doctor` to check PATH
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm nested`

vpx fails the same way for system-only npm when the first match is the npm bundled with the inherited managed runtime.

**Exit code:** 1

```
error: vpx: VP_NPM_MANAGER=system-only rejects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm' that PATH selects for 'npm'; run `vp env doctor` to check PATH
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm --version`

Without a system npm either, the first match is the npm bundled with the inherited managed runtime, so vpx fails with a pointer to `vp env doctor` instead of running it or downloading one through dlx.

**Exit code:** 1

```
error: vpx: VP_NPM_MANAGER=system-only rejects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm' that PATH selects for 'npm'; run `vp env doctor` to check PATH
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm --version`

System-only Node.js with npm in its own (managed) mode: vpx takes npm from PATH, as without system-only, and rejects the npm bundled with the inherited managed runtime, which would run managed Node.js.

**Exit code:** 1

```
error: vpx: VP_NODE_MANAGER=system-only rejects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm' that PATH selects for 'npm'; run `vp env doctor` to check PATH
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_BYPASS=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm --version`

VP_BYPASS hands the npm shim to the first npm on PATH outside the shims, which it rejects the same way.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only rejects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm' that PATH selects for 'npm'; run `vp env doctor` to check PATH
```

## `vp env off npm`

```
VITE+ - The Unified Toolchain for the Web

✓ npm management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `vp env off node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_NODE_MANAGER=system-only VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm nested`

Both system-only with the shims after the system tools: the shell runs system npm on system Node.js, and so does its nested npm, without Vite+.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_NODE_MANAGER=system-only VP_NPM_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm nested`

vpx selects the same system npm as the first PATH match.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vp env exec npm nested`

System-only Node.js with system-first npm: vp env exec takes npm from PATH, as without system-only, and it is not a managed runtime.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vp env exec npm --version`

The same lookup rejects the npm bundled with an inherited managed runtime, which would run managed Node.js.

**Exit code:** 1

```
error: VP_NODE_MANAGER=system-only rejects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm' that PATH selects for 'npm'; run `vp env doctor` to check PATH
```

## `vp env on npm`

```
VITE+ - The Unified Toolchain for the Web

✓ npm management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm nested`

System-only Node.js with managed npm: the npm shim runs the npm beside the first Node.js on PATH, the system one, and so does the nested npm through the shim.

```
system npm on system Node.js
system npm child on system Node.js
```
