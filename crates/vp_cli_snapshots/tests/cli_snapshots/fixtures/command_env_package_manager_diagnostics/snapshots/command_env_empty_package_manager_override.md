# command_env_empty_package_manager_override

## `VP_PACKAGE_MANAGER=    vp env current pm --json`

an empty package-manager environment override falls through to project resolution

```
{
  "package_manager": {
    "name": "npm",
    "version": "<version>",
    "source": "packageManager",
    "source_path": "<workspace>/package.json",
    "project_root": "<workspace>",
    "bin_paths": {
      "npm": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npm",
      "npx": "<home>/.vite-plus/package_manager/npm/<version>/npm/bin/npx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```
