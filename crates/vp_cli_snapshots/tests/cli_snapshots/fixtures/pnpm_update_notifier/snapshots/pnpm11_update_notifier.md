# pnpm11_update_notifier

Managed pnpm 11 commands disable update notifications even when the project enables them.

## `vp pm config get updateNotifier`

```
false
```

## `vp install`

```
VITE+ - The Unified Toolchain for the Web

Already up to date
. postinstall$ node -e "console.log('PNPM_CONFIG_UPDATE_NOTIFIER=' + process.env.PNPM_CONFIG_UPDATE_NOTIFIER)"
│ PNPM_CONFIG_UPDATE_NOTIFIER=false
└─ Done in <duration>

Done in <duration> using pnpm <version>
```
