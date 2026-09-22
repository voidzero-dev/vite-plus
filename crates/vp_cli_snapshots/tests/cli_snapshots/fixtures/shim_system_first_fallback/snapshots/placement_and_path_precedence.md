# placement_and_path_precedence

## `node setup-system-node.cjs`


## `vp env on`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node list-shims.cjs $VP_HOME/bin $VP_HOME/fallback-bin`

Managed mode keeps all tool families in the main bin; fallback can be empty.

```
$VP_HOME/bin: bun, bunx, node, npm, npx, pn, pnpm, pnpx, pnx, vpr, vpx, yarn, yarnpkg
$VP_HOME/fallback-bin: (empty)
```

## `vp env off node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node list-shims.cjs $VP_HOME/bin $VP_HOME/fallback-bin`

Only Node moves to fallback.

```
$VP_HOME/bin: bun, bunx, npm, npx, pn, pnpm, pnpx, pnx, vpr, vpx, yarn, yarnpkg
$VP_HOME/fallback-bin: node
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} node -p process.execPath.includes('system-bin')`

The shell selects system Node before fallback.

```
true
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} node assert-node-resolution.cjs`

Internal Node resolution agrees with PATH.

```
env which selects the same Node as PATH
```

## `vp env off pnpm`

```
VITE+ - The Unified Toolchain for the Web

✓ pnpm management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node list-shims.cjs $VP_HOME/bin $VP_HOME/fallback-bin`

pnpm and pnpx move together; other families and vpx/vpr stay managed.

```
$VP_HOME/bin: bun, bunx, npm, npx, vpr, vpx, yarn, yarnpkg
$VP_HOME/fallback-bin: node, pn, pnpm, pnpx, pnx
```

## `vp env setup --refresh`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node list-shims.cjs $VP_HOME/bin $VP_HOME/fallback-bin`

Refresh preserves saved choices.

```
$VP_HOME/bin: bun, bunx, npm, npx, vpr, vpx, yarn, yarnpkg
$VP_HOME/fallback-bin: node, pn, pnpm, pnpx, pnx
```

## `PATH=${VP_HOME}/fallback-bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} node -p process.execPath.includes('system-bin')`

A fallback shim before system Node selects managed Node.

```
false
```

## `PATH=${VP_HOME}/fallback-bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} node assert-node-resolution.cjs`

Internal resolution must also stop at the first Vite+ shim.

```
env which selects the same Node as PATH
```

## `vp env on node`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node list-shims.cjs $VP_HOME/bin $VP_HOME/fallback-bin`

Restoring Node leaves the pnpm preference intact and removes its old fallback entry.

```
$VP_HOME/bin: bun, bunx, node, npm, npx, vpr, vpx, yarn, yarnpkg
$VP_HOME/fallback-bin: pn, pnpm, pnpx, pnx
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} node -p process.execPath.includes('system-bin')`

Managed Node takes precedence again.

```
false
```

## `vp env on pnpm`

```
VITE+ - The Unified Toolchain for the Web

✓ pnpm management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${VP_HOME}/fallback-bin${PATH_SEPARATOR}${PATH} node list-shims.cjs $VP_HOME/bin $VP_HOME/fallback-bin`

Restoring pnpm empties fallback again.

```
$VP_HOME/bin: bun, bunx, node, npm, npx, pn, pnpm, pnpx, pnx, vpr, vpx, yarn, yarnpkg
$VP_HOME/fallback-bin: (empty)
```
