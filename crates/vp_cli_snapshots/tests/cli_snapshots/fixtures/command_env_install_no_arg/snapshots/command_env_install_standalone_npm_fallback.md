# command_env_install_standalone_npm_fallback

Explicit npm installation uses standalone registry npm without installing Node.js.

## `vp env use npm --no-install`

an explicit npm scope exports the standalone npm fallback

```
export VP_NPM_VERSION=12.0.2
Using npm <version> (resolved from registry fallback)
```

## `vp env install npm`

an explicit npm scope installs the standalone registry fallback

```
VITE+ - The Unified Toolchain for the Web

Installing npm <version>...
Installed npm <version>
```

## `vpt stat-file $VP_HOME/js_runtime/node --assert missing`

installing standalone npm does not install Node.js

```
<home>/.vite-plus/js_runtime/node: missing
```

## `vp env install 22.18.0`


## `VP_NODE_VERSION=22.18.0 node assert-installed-npm.cjs`

the standalone npm fallback is installed but is not selected

```
Standalone npm is installed but is not current
```

## `VP_NODE_VERSION=22.18.0 vp env current npm --json`

installing standalone npm does not select it: current reports Node's bundled npm until a version is configured

```
{
  "package_manager": {
    "name": "npm",
    "version": "<version>",
    "source": "Node.js bundled npm",
    "bin_paths": {
      "npm": "<home>/.vite-plus/js_runtime/node/<version>/bin/npm",
      "npx": "<home>/.vite-plus/js_runtime/node/<version>/bin/npx"
    },
    "installed": true,
    "mode": "managed"
  }
}
```
