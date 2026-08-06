# fallback_all_invalid_to_user_default

## `vp env default 22.18.0`

Set user default

```
VITE+ - The Unified Toolchain for the Web

✓ Default Node.js version set to 22.18.0
```

## `vp exec node -e console.log(process.version)`

Should use default 22.18.0, not LTS

```
VITE+ - The Unified Toolchain for the Web

warning: invalid version 'invalid' in engines.node, ignoring
<version>
```
