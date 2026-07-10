# migration_existing_husky_v8_multi_hooks

## `git init`


## `vp migrate --no-interactive`

should warn about husky v8 and skip hooks setup

```
VITE+ - The Unified Toolchain for the Web

⚠ Detected husky <9.0.0 — please upgrade to husky v9+ first, then re-run migration.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

husky/lint-staged should remain in devDeps, prepare should stay as husky

```
{
  "name": "migration-existing-husky-v8-multi-hooks",
  "scripts": {
    "prepare": "husky"
  },
  "devDependencies": {
    "husky": "^8.0.0",
    "lint-staged": "^15.0.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "devEngines": {
    "packageManager": {
      "name": "pnpm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml has overrides and catalog

```
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
overrides:
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vpt print-file .husky/pre-commit`

hook file should be unchanged (still has bootstrap)

```
. "$(dirname -- "$0")/_/husky.sh"

npx lint-staged
```

## `vpt print-file .husky/commit-msg`

hook file should be unchanged (still has bootstrap)

```
. "$(dirname -- "$0")/_/husky.sh"

npx commitlint --edit $1
```
