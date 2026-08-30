# command_env_package_manager_modes

## `vp env off pnpm`

switch only pnpm to system-first mode

```
VITE+ - The Unified Toolchain for the Web

✓ pnpm management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `VP_PACKAGE_MANAGER=pnpm@10.18.0 VP_BYPASS=${PATH} vp env current pnpm --json`

pnpm uses its individual mode

```
{
  "package_manager": {
    "name": "pnpm",
    "version": "<version>",
    "source": "VP_PACKAGE_MANAGER",
    "bin_paths": {
      "pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm",
      "pnpx": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx"
    },
    "installed": false,
    "mode": "system_first"
  }
}
```

## `VP_PACKAGE_MANAGER=bun@1.2.3 vp env current bun --json`

bun keeps the shared managed mode

```
{
  "package_manager": {
    "name": "bun",
    "version": "<version>",
    "source": "VP_PACKAGE_MANAGER",
    "bin_paths": {
      "bun": "<home>/.vite-plus/package_manager/bun/<version>/bun/bin/bun",
      "bunx": "<home>/.vite-plus/package_manager/bun/<version>/bun/bin/bunx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```

## `vp env on pnpm`

restore only pnpm to managed mode

```
VITE+ - The Unified Toolchain for the Web

✓ pnpm management set to managed.

Selected commands and shims will now use Vite+ managed tools.

Run `vp env off` to prefer system tools instead.
```

## `VP_PACKAGE_MANAGER=pnpm@10.18.0 vp env current pnpm --json`

pnpm returns to the shared managed mode

```
{
  "package_manager": {
    "name": "pnpm",
    "version": "<version>",
    "source": "VP_PACKAGE_MANAGER",
    "bin_paths": {
      "pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm",
      "pnpx": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```
