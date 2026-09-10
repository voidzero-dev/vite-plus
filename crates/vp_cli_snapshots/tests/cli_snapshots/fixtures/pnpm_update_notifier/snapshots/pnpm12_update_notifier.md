# pnpm12_update_notifier

Managed pnpm 12 commands disable update notifications even when the project enables them.

## `vpt json-edit package.json packageManager pnpm@12.3.4`


## `vp pm config get updateNotifier`

```
false
```

## `vp install`

```
VITE+ - The Unified Toolchain for the Web

. postinstall$ node -e "console.log('PNPM_CONFIG_UPDATE_NOTIFIER=' + process.env.PNPM_CONFIG_UPDATE_NOTIFIER)"
│ PNPM_CONFIG_UPDATE_NOTIFIER=false
└─ Done in <duration>
Already up to date

Done in <duration> using pnpm <version>
```
