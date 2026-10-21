# command_self_setup_homebrew_user_dependencies

## `node homebrew.mjs`

```
First use copies the release lockfile and records a successful frozen install.
Authenticated first use retries once, preserves preferences, and serializes concurrent setup.
CLI, Node, npm, pn, and pnx shims survive Cellar replacement and removal.
A failed frozen refresh invalidates completion and retries with saved preferences.
Ownership commands use the tap name; migration preserves global packages and refreshes their shims.
User installations and cleanup stay independent.
Relative, tilde, and environment-variable selectors reach pnpm as the same absolute user config.
Registry overrides and basic authentication work with the shared downloader.
Missing authentication and corrupted pnpm tarballs stop setup.
```
