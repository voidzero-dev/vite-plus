# migration_composed_husky_custom_dir

## `git init`


## `vpt mkdir -p .config/husky/_`


## `vpt write-file .config/husky/pre-commit '#'\!'/usr/bin/env sh
npx lint-staged
'`


## `vpt write-file .config/husky/_/h '#'\!'/usr/bin/env sh
echo custom dispatcher
'`


## `vp migrate --no-interactive`

migration should skip the nonstandard Husky setup

```
VITE+ - The Unified Toolchain for the Web

⚠ Nonstandard Husky command detected in scripts.prepare — skipping git hooks setup. Vite+ only migrates conventional .husky setups; configure hooks manually.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

prepare and the Husky dependency should be preserved

```
{
  "name": "migration-composed-husky-custom-dir",
  "scripts": {
    "prepare": "npm run build && husky install .config/husky"
  },
  "devDependencies": {
    "husky": "^9.1.7",
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

## `vpt print-file .config/husky/pre-commit`

custom hook should be unchanged

```
#!/usr/bin/env sh
npx lint-staged
```

## `vpt print-file .config/husky/_/h`

custom dispatcher should be unchanged

```
#!/usr/bin/env sh
echo custom dispatcher
```
