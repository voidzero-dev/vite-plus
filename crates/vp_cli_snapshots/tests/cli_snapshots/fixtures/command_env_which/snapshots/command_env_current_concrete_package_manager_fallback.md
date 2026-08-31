# command_env_current_concrete_package_manager_fallback

A named family has an effective registry fallback even when no project, session, or default selection exists.

## `vp env current pnpm --json`

a concrete family reports the same registry fallback as its shim

```
{
  "package_manager": {
    "name": "pnpm",
    "version": "<version>",
    "source": "registry fallback",
    "bin_paths": {
      "pnpm": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpm",
      "pnpx": "<home>/.vite-plus/package_manager/pnpm/<version>/pnpm/bin/pnpx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```
