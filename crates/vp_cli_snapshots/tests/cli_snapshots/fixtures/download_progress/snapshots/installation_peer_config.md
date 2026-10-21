# installation_peer_config

## `node peer-config.mjs`

```
The generator pins all frozen-lockfile settings despite user and environment values.
Without the installer override, real pnpm rejects auto-install-peers.
Frozen setup overrides auto-install-peers in user and environment configuration.
Without the installer override, real pnpm rejects peers-suffix-max-length.
Frozen setup overrides peers-suffix-max-length in user and environment configuration.
Without the installer override, real pnpm rejects exclude-links-from-lockfile.
Frozen setup overrides exclude-links-from-lockfile in user and environment configuration.
Without the installer override, real pnpm rejects dedupe-peers.
Frozen setup overrides dedupe-peers in user and environment configuration.
Without the installer override, real pnpm rejects inject-workspace-packages.
Frozen setup overrides inject-workspace-packages in user and environment configuration.
Without the installer override, real pnpm rejects lockfile.
Frozen setup overrides lockfile in user and environment configuration.
All installs fetch the CLI and its peer using the user registry and authentication.
The published lockfile and user configuration remain unchanged.
```
