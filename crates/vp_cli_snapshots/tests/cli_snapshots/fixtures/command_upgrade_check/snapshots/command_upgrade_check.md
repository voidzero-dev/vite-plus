# command_upgrade_check

## `vp upgrade --check --tag alpha`

alpha tag avoids release-day flake (dev version equals npm latest right after a release, hiding the Update-available branch)

```
info: checking for updates...
info: found vite-plus@0.1.21-alpha.7 (current: <version>)
Update available: <version> → 0.1.21-alpha.7
Run `vp upgrade` to update.
```
