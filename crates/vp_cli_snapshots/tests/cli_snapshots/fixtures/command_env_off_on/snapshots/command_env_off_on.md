# command_env_off_on

## `vp run assert-managed`

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

## `vp run assert-not-managed`

System-first mode: must NOT use 22.18.0

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

$ node src/assert-not-managed.mjs ⊘ cache disabled
Expected system Node.js, got managed <version>
```

## `vp env on`

Switch back to managed mode

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `vp run assert-managed`

Managed mode restored: should use 22.18.0 again

```
VITE+ - The Unified Toolchain for the Web

$ node src/assert-managed.mjs ⊘ cache disabled
OK: <version>
```
