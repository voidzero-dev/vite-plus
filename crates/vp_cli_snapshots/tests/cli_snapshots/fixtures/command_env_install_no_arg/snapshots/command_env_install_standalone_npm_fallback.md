# command_env_install_standalone_npm_fallback

## `vp env install npm`

an explicit npm scope installs the standalone registry fallback

```
VITE+ - The Unified Toolchain for the Web

Installing npm <version>...
Installed npm <version>
```

## `vp env current npm --json`

the standalone npm fallback is installed

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
    "installed": true,
    "mode": "managed"
  }
}
```

## `vpt stat-file $VP_HOME/js_runtime/node --assert missing`

installing standalone npm does not install Node.js

```
<home>/.vite-plus/js_runtime/node: missing
```
