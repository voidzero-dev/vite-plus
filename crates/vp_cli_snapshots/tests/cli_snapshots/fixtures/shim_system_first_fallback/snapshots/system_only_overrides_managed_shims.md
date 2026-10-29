# system_only_overrides_managed_shims

With managed config.json, VP_*_MANAGER=system-only makes the shims use only system tools: no downloads, no network, no cached managed installs, and an unchanged child PATH.

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

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_VERSION=24.0.0 VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node -p process.execPath.includes('system-bin')`

The node shim runs system Node.js, ignoring the session override and the cached managed runtime.

```
true
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} EXPECTED_PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node -p 'process.env.PATH === process.env.EXPECTED_PATH'`

The system Node.js directory is not moved to the front of the child PATH.

```
true
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} EXPECTED_PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm --version`

npm follows its own (managed) mode: without an npm pin it is the npm beside system Node.js, and the child PATH is unchanged.

```
system npm --version
same PATH
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PNPM_MANAGER=system-only VP_PM_MANAGER=yes NPM_CONFIG_REGISTRY=http://127.0.0.1:1 pnpm --version`

VP_PNPM_MANAGER=system-only wins over VP_PM_MANAGER=yes and over the packageManager pin. Node.js stays managed, so only its directory is added to the child PATH.

```
system pnpm --version
changed PATH
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_PNPM_MANAGER=yes VP_PM_MANAGER=system-only NPM_CONFIG_REGISTRY=http://127.0.0.1:1 npm --version`

VP_PM_MANAGER=system-only applies to families without their own value.

```
system npm --version
changed PATH
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} VP_NODE_MANAGER=System-Only node -p process.execPath.includes('system-bin')`

Only the exact lowercase value is system-only; managed config.json stays in effect.

```
false
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} NEST_DEPTH=8 VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node nest-node.cjs`

Nested calls with the same arguments and cwd still run through 8 shim entries.

```
NESTED_NODE_OK
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} NEST_DEPTH=9 VP_NODE_MANAGER=system-only VP_NODE_DIST_MIRROR=http://127.0.0.1:1 NPM_CONFIG_REGISTRY=http://127.0.0.1:1 node nest-node.cjs`

A 9th identical entry is treated as a forwarding loop, so the count reaches nested shims.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only requires a system 'node' on PATH that does not forward back to Vite+, but the one in '<workspace>/system-bin' re-entered the shim more than 8 times with the same arguments and directory
```

## `node list-managed.cjs`

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

## `PATH=${VP_HOME}/bin VP_NODE_MANAGER=system-only node --version`

Without a system Node.js on PATH, the shim fails instead of using the managed runtime.

**Exit code:** 1

```
vp: VP_NODE_MANAGER=system-only requires a system 'node' on PATH, but none was found
```
