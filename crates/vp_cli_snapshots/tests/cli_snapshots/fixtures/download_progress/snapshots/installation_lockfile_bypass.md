# installation_lockfile_bypass

## `node peer-config.mjs bypass`

```
The generator pins all frozen-lockfile settings despite user and environment values.
A broken lockfile still fails unless VP_SETUP_SKIP_INSTALL_LOCKFILE is exactly 1.
The bypass warns, ignores broken or missing lockfiles, and installs the exact CLI version.
User registry and authentication still work; the completion receipt is explicitly unlocked.
Upgrade rejects a missing companion by default and succeeds with the bypass.
Upgrade handoff completes setup; later commands work without the override.
Normal setup restores the published lockfile and records a fresh frozen install.
A failed bypass clears any stale receipt and leaves setup incomplete.
```
