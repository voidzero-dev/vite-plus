# fallback_invalid_engines_to_dev_engines

## `vp exec node -e console.log(process.version)`

Should use devEngines.runtime 22.18.0, not LTS

```
VITE+ - The Unified Toolchain for the Web

warning: invalid version 'invalid' in engines.node, ignoring
<version>
```

## `vp env which node`

Should show devEngines.runtime source

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/js_runtime/node/<version>/bin/node
  Version:    22.18.0
  Source:     <workspace>/package.json
```
