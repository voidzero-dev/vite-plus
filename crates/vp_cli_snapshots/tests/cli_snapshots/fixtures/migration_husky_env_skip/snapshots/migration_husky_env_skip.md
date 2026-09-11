# migration_husky_env_skip

## `git init`


## `vp migrate --no-interactive`

with HUSKY=0, vp config should skip and warn instead of reporting success

```
VITE+ - The Unified Toolchain for the Web

⚠ Detected Husky — leaving its hooks, configuration, and dependencies unchanged. Migrate Husky manually before enabling Vite+ hooks.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```
