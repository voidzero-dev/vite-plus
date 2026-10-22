# command_upgrade_check

## `vp upgrade --check`

mock-manifest.json pins `latest` to 999.0.0, so the Update-available branch is deterministic.

```
info: checking for updates...
info: found vite-plus@999.0.0 (current: <version>)
Update available: <version> → 999.0.0
Run `vp upgrade` to update.
```
