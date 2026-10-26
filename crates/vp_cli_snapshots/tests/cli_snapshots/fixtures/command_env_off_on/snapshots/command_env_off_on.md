# command_env_off_on

`vp env off` switches project scripts from the managed engines.node runtime to the system Node.js on PATH; `vp env on` switches back.

## `node src/setup-system-node.cjs`

Provide a system Node.js after the Vite+ bin on PATH, as in a user's shell.


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp run assert-managed`

Managed mode: should use project's engines.node 22.18.0

```
VITE+ - The Unified Toolchain for the Web

$ node src/assert-managed.mjs ⊘ cache disabled
OK: <version>
```

## `vp env off`

Switch to system-first mode

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp run assert-not-managed`

System-first mode: must use the system Node.js, not 22.18.0

```
VITE+ - The Unified Toolchain for the Web

$ node src/assert-not-managed.mjs ⊘ cache disabled
OK: system <version>
```

## `vp env on`

Switch back to managed mode

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} vp run assert-managed`

Managed mode restored: should use 22.18.0 again

```
VITE+ - The Unified Toolchain for the Web

$ node src/assert-managed.mjs ⊘ cache disabled
OK: <version>
```
