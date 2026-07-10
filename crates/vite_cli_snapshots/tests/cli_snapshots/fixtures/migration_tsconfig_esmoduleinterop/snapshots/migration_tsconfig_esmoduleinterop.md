# migration_tsconfig_esmoduleinterop

## `vp migrate --no-interactive`

should remove esModuleInterop: false from tsconfig.json

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 4 config updates applied
! Warnings:
  - Removed `"esModuleInterop": false` from tsconfig.json — this option has been deprecated. See https://github.com/oxc-project/tsgolint/issues/351, https://github.com/microsoft/TypeScript/issues/62529
  - Removed `"allowSyntheticDefaultImports": false` from tsconfig.json — this option has been deprecated. See https://github.com/oxc-project/tsgolint/issues/351, https://github.com/microsoft/TypeScript/issues/62529
```

## `vpt print-file tsconfig.json`

verify esModuleInterop: false is removed

```
{
  "compilerOptions": {
    "target": "ES2023",
    "module": "ESNext",
    "strict": true
  }
}
```

## `vpt print-file vite.config.ts`

check vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
});
```

## `vpt print-file package.json`

check package.json

```
{
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
  },
  "scripts": {
    "prepare": "vp config"
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
