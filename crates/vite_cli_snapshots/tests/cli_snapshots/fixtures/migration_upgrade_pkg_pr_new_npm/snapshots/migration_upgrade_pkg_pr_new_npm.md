# migration_upgrade_pkg_pr_new_npm

## `vp migrate --no-interactive`

bridge commit builds replace every stale managed spec

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  0.1.20 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

direct dependencies and npm overrides use the same immutable commit version

```
{
  "name": "migration-upgrade-pkg-pr-new-npm",
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "packageManager": "npm@11.11.1"
}
```

## `vp migrate --no-interactive`

bridge commit migration is idempotent

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

rerun leaves package.json unchanged (identical to the first migration)

```
{
  "name": "migration-upgrade-pkg-pr-new-npm",
  "devDependencies": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vite-plus": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  },
  "packageManager": "npm@11.11.1"
}
```
