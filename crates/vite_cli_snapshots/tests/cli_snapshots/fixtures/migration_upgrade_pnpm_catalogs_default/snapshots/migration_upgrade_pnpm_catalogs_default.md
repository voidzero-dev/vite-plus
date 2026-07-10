# migration_upgrade_pnpm_catalogs_default

## `vp migrate --no-interactive`

reuse the managed named catalog beside catalogs.default

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.20 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

existing catalog:build dependency references are preserved

```
{
  "name": "migration-upgrade-pnpm-catalogs-default",
  "devDependencies": {
    "vite": "catalog:build",
    "vite-plus": "catalog:build"
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

catalogs.default remains the only default catalog definition

```
packages:
  - .

catalogs:
  build:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vite-plus: <version>
  default:
    rari: ^0.14.12
overrides:
  vite: catalog:build
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vp migrate --no-interactive`

catalogs.default migration is idempotent

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

rerun leaves package.json unchanged

```
{
  "name": "migration-upgrade-pnpm-catalogs-default",
  "devDependencies": {
    "vite": "catalog:build",
    "vite-plus": "catalog:build"
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

rerun leaves catalog placement unchanged

```
packages:
  - .

catalogs:
  build:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vite-plus: <version>
  default:
    rari: ^0.14.12
overrides:
  vite: catalog:build
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
