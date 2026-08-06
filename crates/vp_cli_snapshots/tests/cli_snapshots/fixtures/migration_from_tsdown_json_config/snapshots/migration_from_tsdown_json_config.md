# migration_from_tsdown_json_config

## `vp migrate --no-interactive`

migration should rewrite imports to vite-plus

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied
```

## `vpt stat-file tsdown.config.json --assert-not file`

check tsdown.config.json should be removed

```
tsdown.config.json: missing
```

## `vpt print-file vite.config.ts`

check vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  pack: {
    "entry": "src/index.ts",
    "outDir": "dist",
    "format": ["esm", "cjs"],
    "dts": true,
    "inputOptions": {
      "cwd": "./src"
    }
  },
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  server: {
    port: 3000,
  },
});
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-from-tsdown-json-config",
  "scripts": {
    "build": "vp pack",
    "build:watch": "vp pack --watch",
    "build:dts": "vp pack --dts",
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

## `vp migrate --no-interactive`

run migration again to check if it is idempotent

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

check vite.config.ts

```
import { defineConfig } from 'vite-plus';

export default defineConfig({
  staged: {
    "*": "vp check --fix"
  },
  pack: {
    "entry": "src/index.ts",
    "outDir": "dist",
    "format": ["esm", "cjs"],
    "dts": true,
    "inputOptions": {
      "cwd": "./src"
    }
  },
  fmt: {},
  lint: {"jsPlugins":[{"name":"vite-plus","specifier":"vite-plus/oxlint-plugin"}],"rules":{"vite-plus/prefer-vite-plus-imports":"error"},"options":{"typeAware":true,"typeCheck":true}},
  server: {
    port: 3000,
  },
});
```

## `vpt print-file package.json`

check package.json

```
{
  "name": "migration-from-tsdown-json-config",
  "scripts": {
    "build": "vp pack",
    "build:watch": "vp pack --watch",
    "build:dts": "vp pack --dts",
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
