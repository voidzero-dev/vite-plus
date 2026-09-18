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

## `vp env list npm --json`

the standalone npm fallback is installed

```
{
  "package_managers": {
    "npm": [
      {
        "version": "12.0.2",
        "current": true,
        "default": false
      }
    ]
  }
}
```

## `vpt stat-file $VP_HOME/js_runtime/node --assert missing`

installing standalone npm does not install Node.js

```
<home>/.vite-plus/js_runtime/node: missing
```
