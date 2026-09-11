# migration_hooks_skip_on_existing_hookspath

## `git init`


## `git config core.hooksPath .custom-hooks`


## `vp migrate --no-interactive`

should skip hooks because core.hooksPath is already set

```
VITE+ - The Unified Toolchain for the Web

⚠ core.hooksPath is already set to ".custom-hooks" — leaving the existing hook setup unchanged.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

the package should not gain hook policy or lifecycle changes

```
{
  "name": "migration-hooks-skip-on-existing-hookspath",
  "devDependencies": {
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
  vite@*: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `git config --local core.hooksPath`

should still be .custom-hooks

```
.custom-hooks
```
