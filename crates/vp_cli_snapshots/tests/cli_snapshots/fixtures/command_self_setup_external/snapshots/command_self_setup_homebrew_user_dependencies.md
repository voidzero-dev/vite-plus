# command_self_setup_homebrew_user_dependencies

## `node homebrew.mjs`

```
Authenticated first use retries once, preserves preferences, and serializes concurrent setup.
CLI, Node, npm, pn, and pnx shims survive Cellar replacement and removal.
Ownership commands use the tap name; migration preserves global packages and refreshes their shims.
User installations and cleanup stay independent.
Registry overrides, relative user configuration, and basic authentication work with bundled npm.
Missing authentication and corrupted pnpm tarballs stop setup.
```
