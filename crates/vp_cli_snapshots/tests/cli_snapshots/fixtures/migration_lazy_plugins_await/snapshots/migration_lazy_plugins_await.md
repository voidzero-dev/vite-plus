# migration_lazy_plugins_await

## `vp migrate --no-interactive --no-hooks`

migration should wrap awaited inline plugins with async lazyPlugins

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied, 1 file had imports rewritten
• Inline Vite plugins wrapped with lazyPlugins for check/lint/fmt
```

## `vpt print-file vite.config.ts`

check awaited plugins use async lazyPlugins

```
import react from '@vitejs/plugin-react';
import { defineConfig, lazyPlugins } from 'vite-plus';

async function loadPlugin() {
  return { name: 'loaded-plugin' };
}

export default defineConfig({
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  plugins: lazyPlugins(async () => [react(), await loadPlugin()]),
});
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-lazy-plugins-await",
  "scripts": {
    "dev": "vp dev",
    "build": "vp build"
  },
  "devDependencies": {
    "@vitejs/plugin-react": "^4.2.0",
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "pnpm@10.33.2"
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
