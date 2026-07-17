# migration_prettier_pkg_json

## `vp migrate --no-interactive`

migration should detect prettier in package.json and auto-migrate

```
VITE+ - The Unified Toolchain for the Web

Prettier configuration detected. Auto-migrating to Oxfmt...
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied
• Prettier migrated to Oxfmt
```

## `vpt print-file package.json`

check prettier key removed, scripts rewritten, dep removed

```
{
  "name": "migration-prettier-pkg-json",
  "scripts": {
    "format": "vp fmt .",
    "format:check": "vp fmt --check .",
    "prepare": "vp config"
  },
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
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vpt print-file vite.config.ts`

check oxfmt config merged into vite.config.ts with semi/singleQuote settings

```
import { defineConfig } from "vite-plus";

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  fmt: {
    semi: true,
    singleQuote: true,
    printWidth: 80,
    sortPackageJson: false,
    ignorePatterns: [],
  },
});
```
