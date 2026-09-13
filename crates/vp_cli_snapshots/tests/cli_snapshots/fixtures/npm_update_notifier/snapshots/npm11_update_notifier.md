# npm11_update_notifier

Managed npm 11 commands disable update notifications even when the project enables them.

## `vp pm config get update-notifier`

```
false
```

## `vp install`

```
VITE+ - The Unified Toolchain for the Web

> npm-update-notifier@1.0.0 postinstall
> node -e "console.log('npm_config_update_notifier=' + process.env.npm_config_update_notifier)"

npm_config_update_notifier=false

up to date in <duration>
```
