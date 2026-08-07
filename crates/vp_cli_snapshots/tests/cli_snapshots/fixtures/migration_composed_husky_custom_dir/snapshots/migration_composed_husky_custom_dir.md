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

migration should skip the custom Husky setup

```
VITE+ - The Unified Toolchain for the Web

⚠ Detected Husky — leaving its hooks, configuration, and dependencies unchanged. Migrate Husky manually before enabling Vite+ hooks.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
```

## `vpt print-file package.json`

prepare and the Husky dependency should remain

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

## `vpt stat-file .vite-hooks --assert-not dir`

no Vite+ hook tree should be created

```
.vite-hooks: missing
```
