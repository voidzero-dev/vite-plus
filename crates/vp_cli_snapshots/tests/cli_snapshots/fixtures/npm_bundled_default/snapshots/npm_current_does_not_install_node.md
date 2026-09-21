# npm_current_does_not_install_node

## `vp env current --json`

An absent Node runtime and bundled npm are reported without downloading

```
{
  "node": {
    "version": "22.18.0",
    "source": "VP_NODE_VERSION",
    "bin_path": "<home>/.vite-plus/js_runtime/node/<version>/bin/node",
    "installed": false,
    "mode": "managed"
  },
  "package_manager": {
    "name": "npm",
    "version": "unknown",
    "source": "Node.js bundled npm",
    "project_root": "<workspace>",
    "bin_paths": {
      "npm": "<home>/.vite-plus/js_runtime/node/<version>/bin/npm",
      "npx": "<home>/.vite-plus/js_runtime/node/<version>/bin/npx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```

## `vp env current npm --json`

The explicit npm scope also reports an unknown, uninstalled version

```
{
  "package_manager": {
    "name": "npm",
    "version": "unknown",
    "source": "Node.js bundled npm",
    "bin_paths": {
      "npm": "<home>/.vite-plus/js_runtime/node/<version>/bin/npm",
      "npx": "<home>/.vite-plus/js_runtime/node/<version>/bin/npx"
    },
    "installed": false,
    "mode": "managed"
  }
}
```

## `vpt stat-file $VP_HOME/js_runtime/node/22.18.0 --assert missing`

Reporting the environment leaves the runtime uninstalled

```
<home>/.vite-plus/js_runtime/node/<version>: missing
```
