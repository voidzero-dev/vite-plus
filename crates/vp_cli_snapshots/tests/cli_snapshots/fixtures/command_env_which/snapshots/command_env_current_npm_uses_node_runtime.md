# command_env_current_npm_uses_node_runtime

## `vp env current npm --json`

unselected npm reports the executable bundled with the resolved Node.js runtime

```
{
  "package_manager": {
    "name": "npm",
    "version": "<version>",
    "source": "Node.js <version>",
    "source_path": "<workspace>/.node-version",
    "project_root": "<workspace>",
    "bin_paths": {
      "npm": "<home>/.vite-plus/js_runtime/node/<version>/bin/npm",
      "npx": "<home>/.vite-plus/js_runtime/node/<version>/bin/npx"
    },
    "installed": true,
    "mode": "managed"
  }
}
```
