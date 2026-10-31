# system_only_overrides_managed_shims

VP_*_MANAGER=system-only shims never search PATH or use managed installs. With managed config.json they fail and suggest `vp env off`; system tools still run when they come before the shims, as in a devShell, or after `vp env off`. Nothing is downloaded and the child PATH is unchanged.

## `node setup-system-node.cjs`


## `node setup-system-tools.cjs`


## `vpt write-file package.json '{"packageManager":"pnpm@10.18.0"}'`


## `node list-managed.cjs`

Managed installs before the system-only steps: the pinned runtime is cached, and the override is not.

```
managed Node.js (.node-version pin): true
managed Node.js (VP_NODE_VERSION override): false
package_manager: (empty)
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node --version`

The node shim fails without searching PATH for the system Node.js after it, and suggests `vp env off node`.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only requires a system 'node', but the Vite+ shim comes first on PATH; run `vp env off node` to move the shims after the system tools
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_BYPASS=1 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node --version`

VP_BYPASS does not make it search PATH either.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only requires a system 'node', but the Vite+ shim comes first on PATH; run `vp env off node` to move the shims after the system tools
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PNPM_MANAGER=system-only VP_PM_MANAGER=yes NPM_CONFIG_REGISTRY=http://127.0.0.1:1 pnpm --version`

VP_PNPM_MANAGER=system-only wins over VP_PM_MANAGER=yes and over the packageManager pin, so the pnpm shim fails the same way.

**Exit code:** 1

```
vp: VP_PNPM_MANAGER=system-only requires a system 'pnpm', but the Vite+ shim comes first on PATH; run `vp env off pnpm` to move the shims after the system tools
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PNPM_MANAGER=yes VP_PM_MANAGER=system-only NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm --version`

VP_PM_MANAGER=system-only applies to families without their own value.

**Exit code:** 1

```
vp: VP_PM_MANAGER=system-only requires a system 'npm', but the Vite+ shim comes first on PATH; run `vp env off npm` to move the shims after the system tools
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=System-Only node -p process.execPath.includes('system-bin')`

Only the exact lowercase value is system-only; managed config.json stays in effect.

```
false
```

## `PATH=${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_VERSION=24.0.0 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vp node -p process.execPath.includes('system-bin')`

vp node runs the system Node.js first on PATH, ignoring the session override.

```
true
```

## `PATH=${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_VERSION=24.0.0 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vpx node -p process.execPath.includes('system-bin')`

vpx node does the same.

```
true
```

## `PATH=${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${PATH} EXPECTED_PATH=${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_VERSION=24.0.0 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 vp env exec npm --version`

npm follows its own (managed) mode: without an npm pin it is the npm beside the system Node.js, and the child PATH is unchanged.

```
system npm --version
same PATH
```

## `vp env off node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} EXPECTED_PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm --version`

The managed npm shim still comes first, and runs the npm beside the first Node.js on PATH, the system one, with the child PATH unchanged.

```
system npm --version
same PATH
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node --version`

Without a system Node.js before the fallback shim, the shim fails instead of using the managed runtime.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only requires a system 'node' on PATH before the Vite+ shims, but none was found
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} node list-managed.cjs`

The system-only steps downloaded nothing.

```
managed Node.js (.node-version pin): true
managed Node.js (VP_NODE_VERSION override): false
package_manager: (empty)
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PNPM_MANAGER=yes VP_PM_MANAGER=system-only node check-managed-pnpm.cjs`

VP_PNPM_MANAGER=yes wins over VP_PM_MANAGER=system-only, so the shim selects the pinned managed pnpm instead of the system one.

```
system pnpm: false
managed pnpm <version> installed: true
```
