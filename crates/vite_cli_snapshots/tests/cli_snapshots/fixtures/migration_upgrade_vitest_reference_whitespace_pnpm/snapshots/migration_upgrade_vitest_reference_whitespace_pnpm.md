# migration_upgrade_vitest_reference_whitespace_pnpm

## `vp migrate --no-interactive`

TypeScript whitespace in a Vitest type directive is valid

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt print-file package.json`

rewritten directive does not retain a redundant Vitest dependency

```
{
  "name": "migration-upgrade-vitest-reference-whitespace-pnpm",
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

## `vpt print-file env.d.ts`

directive is rewritten to the Vite+ public type surface

```
/// <reference types = "vite-plus/test" />
```

## `vpt print-file pnpm-workspace.yaml`

rewritten directive does not retain shared Vitest management

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

directive rewriting is stable on rerun

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
