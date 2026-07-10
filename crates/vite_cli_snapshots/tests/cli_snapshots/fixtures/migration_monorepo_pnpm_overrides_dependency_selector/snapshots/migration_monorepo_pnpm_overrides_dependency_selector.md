# migration_monorepo_pnpm_overrides_dependency_selector

## `vp migrate --no-interactive`

migration should merge pnpm overrides with dependency selector

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
```

## `vpt print-file vite.config.ts`

check vite.config.ts

```
import react from '@vitejs/plugin-react';
import { defineConfig, lazyPlugins } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  plugins: lazyPlugins(() => [react()]),
});
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-monorepo-pnpm-overrides-dependency-selector",
  "version": "1.0.0",
  "scripts": {
    "dev": "vp dev",
    "prepare": "vp config"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "catalog:",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@10.20.0+sha512.cf9998222162dd85864d0a8102e7892e7ba4ceadebbf5a31f9c2fce48dfce317a9c53b9f6464d1ef9042cba2e02ae02a9f7c143a2b438cd93c91840f0192b9dd"
}
```

## `vpt print-file pnpm-workspace.yaml`

check pnpm-workspace.yaml

```
packages:
  - packages/*

catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>

overrides:
  supertest>superagent: 9.0.2
  react-click-away-listener>react: 0.0.0-experimental-7dc903cd-20251203
  vite: 'catalog:'
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vpt print-file packages/app/package.json`

check app package.json

```
{
  "name": "app",
  "scripts": {
    "dev": "vp dev --port 3000",
    "build": "vp build"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "optionalDependencies": {
    "test-vite-plus-other-optional": "1.0.0"
  }
}
```
