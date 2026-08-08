# command_upgrade_background_notice

A background check records an available update without contaminating machine output, then the foreground CLI shows the generic notice at most once per prompt interval.

## `vp upgrade --background-check`


## `vpt grep-file $VP_HOME/cache/upgrade-check.json '"status":"available"'`


## `vp env list --json`

Machine-readable output does not consume the pending notice.


## `vp env off`

The next interactive command displays the cached update notice.

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js management set to system-first.

All vp commands and shims will now prefer system Node.js, falling back to managed if not found.

Run `vp env on` to always use Vite+ managed Node.js.

A new version of vp is available. Run `vp upgrade` to update.
```

## `vp env off`

A subsequent command stays quiet after the notice timestamp is recorded.

```
VITE+ - The Unified Toolchain for the Web

Node.js management is already set to system-first.
All vp commands and shims will prefer system Node.js, falling back to managed if not found.
```
