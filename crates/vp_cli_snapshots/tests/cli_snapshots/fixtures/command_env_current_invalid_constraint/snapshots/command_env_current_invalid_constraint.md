# command_env_current_invalid_constraint

vp env current and vp commands share one version selection, so they report the same invalid lower-priority constraints.

## `vp env current node --json`

a valid .node-version selects the version, and the invalid engines.node below it warns

```
warning: invalid version 'invalid' in engines.node, ignoring
{
  "node": {
    "version": "22.23.4",
    "source": ".node-version",
    "source_path": "<workspace>/.node-version",
    "project_root": "<workspace>",
    "bin_path": "<home>/.vite-plus/js_runtime/node/<version>/bin/node",
    "installed": false,
    "mode": "managed"
  }
}
```
