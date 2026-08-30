# command_env_current_npm_uses_standalone_fallback

## `vp env current npm --json`

an explicit npm scope reports the standalone registry fallback

```
{
  "package_manager": {
    "name": "npm",
    "version": "<version>",
    "source": "registry fallback",
    "bin_paths": {
      "npm": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm",
      "npx": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```
