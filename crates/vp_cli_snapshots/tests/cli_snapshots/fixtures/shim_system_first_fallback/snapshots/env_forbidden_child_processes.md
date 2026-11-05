# env_forbidden_child_processes

VP_ENV_FORBIDDEN=1 holds for what the selected tools run: a `#!/usr/bin/env node` shebang and a nested `npm` resolve from the child PATH. The shims add no directory to it and leave directories inherited from a parent in place; where Vite+ selects a tool from PATH, a managed install there fails. A nested managed npm would print its version instead of the system marker.

## `node setup-system-node.cjs`


## `node setup-child-tools.cjs`


## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PATH_INJECTED_TOOLS=node,npm,npx VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm nested`

The npm shim fails before it would reuse the managed npm that a parent selected, and suggests `vp env off npm`.

**Exit code:** 1

```
vp: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'npm', but the Vite+ shim comes first on PATH; run `vp env off npm` to move the shims after the system tools
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_ENV_FORBIDDEN=1 VP_BYPASS=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node run-shim.cjs npm --version`

VP_BYPASS does not hand the npm shim to the first npm on PATH outside the shims either.

**Exit code:** 1

```
vp: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'npm', but the Vite+ shim comes first on PATH; run `vp env off npm` to move the shims after the system tools
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx node -p process.execPath.includes('system-bin')`

vpx takes Node.js from the first PATH match, the inherited managed runtime, and fails instead of skipping it.

**Exit code:** 1

```
error: vpx: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'node', but PATH selects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/node'; run `vp env doctor` to check PATH
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm nested`

vpx fails the same way for npm when the first match is the npm bundled with the inherited managed runtime.

**Exit code:** 1

```
error: vpx: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'npm', but PATH selects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm'; run `vp env doctor` to check PATH
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm --version`

Without a system npm either, the first match is the npm bundled with the inherited managed runtime, so vpx fails with a pointer to `vp env doctor` instead of running it or downloading one through dlx.

**Exit code:** 1

```
error: vpx: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'npm', but PATH selects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm'; run `vp env doctor` to check PATH
```

## `vp env off npm`

```
VITE+ - The Unified Toolchain for the Web

✓ npm management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm nested`

The shell runs system npm, whose shebang reaches the node shim still first on PATH. The shim fails and suggests `vp env off node` instead of running the managed runtime.

**Exit code:** 1

```
vp: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'node', but the Vite+ shim comes first on PATH; run `vp env off node` to move the shims after the system tools
```

## `vp env off node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm nested`

With the shims after the system tools, the shell runs system npm on system Node.js, and so does its nested npm, without Vite+.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx npm nested`

vpx selects the same system npm as the first PATH match.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vp env exec npm nested`

So does vp env exec.

```
system npm on system Node.js
system npm child on system Node.js
```

## `PATH=${VP_HOME}/js_runtime/node/20.20.2/bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_ENV_FORBIDDEN=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vp env exec npm --version`

vp env exec rejects the npm bundled with an inherited managed runtime, the first PATH match, instead of running it.

**Exit code:** 1

```
error: VP_ENV_FORBIDDEN=1: Vite+ does not manage 'npm', but PATH selects the Vite+ managed install at '<home>/.vite-plus/js_runtime/node/<version>/bin/npm'; run `vp env doctor` to check PATH
```
