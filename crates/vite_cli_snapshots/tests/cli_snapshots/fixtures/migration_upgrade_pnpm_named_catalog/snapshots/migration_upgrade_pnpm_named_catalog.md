# migration_upgrade_pnpm_named_catalog

## `vp migrate --no-interactive`

reuse the existing named-only Vite stack catalog

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.1.21 → <version>
    vite              → <version>
• Package manager settings configured
```

## `vpt print-file package.json`

catalog:vite-stack dependency references are preserved

```
{
  "name": "migration-upgrade-pnpm-named-catalog",
  "devDependencies": {
    "vite": "catalog:vite-stack",
    "vite-plus": "catalog:vite-stack"
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

the bridge commit version is written into vite-stack

```
packages:
  - .

catalogs:
  repo-tooling:
    prettier: 3.8.3
  vite-stack:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vitest: <version>
    vite-plus: <version>
overrides:
  vite: catalog:vite-stack
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```

## `vp migrate --no-interactive`

named-only catalog migration is idempotent

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file package.json`

rerun leaves package.json unchanged

```
{
  "name": "migration-upgrade-pnpm-named-catalog",
  "devDependencies": {
    "vite": "catalog:vite-stack",
    "vite-plus": "catalog:vite-stack"
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
  repo-tooling:
    prettier: 3.8.3
  vite-stack:
    vite: npm:@voidzero-dev/vite-plus-core@<version>
    vitest: <version>
    vite-plus: <version>
overrides:
  vite: catalog:vite-stack
peerDependencyRules:
  allowAny:
    - vite
  allowedVersions:
    vite: '*'
```
