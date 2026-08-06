# delegate_respects_default_node_version

## `vp env default 22.18.0`

Set global default to 22.18.0

```
VITE+ - The Unified Toolchain for the Web

✓ Default Node.js version set to 22.18.0
```

## `vp run check-node`

Should also use 22.18.0

```
VITE+ - The Unified Toolchain for the Web

$ node -e "console.log(process.version)" ⊘ cache disabled
<version>
```

## `vp exec node -e console.log(process.version)`

Should also use 22.18.0

```
VITE+ - The Unified Toolchain for the Web

<version>
```

## `vp env which node`

Should show 22.18.0 from 'default' source

```
VITE+ - The Unified Toolchain for the Web

<home>/.vite-plus/js_runtime/node/<version>/bin/node
  Version:    22.18.0
  Source:     <home>/.vite-plus/config.json
```
