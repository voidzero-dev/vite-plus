# installation_peer_config

## `node peer-config.mjs`

```
The generator pins autoInstallPeers=true despite user and environment settings.
Without the installer override, real pnpm rejects the user configuration.
Frozen setup installs the CLI and its peer using the user registry and authentication.
The published lockfile and user configuration remain unchanged.
```
