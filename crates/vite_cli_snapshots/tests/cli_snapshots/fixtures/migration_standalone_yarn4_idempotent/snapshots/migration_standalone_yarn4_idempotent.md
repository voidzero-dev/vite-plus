# migration_standalone_yarn4_idempotent

## `vp migrate --no-interactive`

implicit Yarn Berry PnP converts before the first pass

```
VITE+ - The Unified Toolchain for the Web

⚠ Vite+ does not currently support Yarn Plug'n'Play (PnP).

✔ Switched Yarn to node-modules mode
◇ Migrated . to Vite+ <version>
• Node <version>  yarn <version>
• 2 config updates applied, 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file package.json`

migrated dependency specs use the Yarn catalog immediately

```
{
  "name": "migration-standalone-yarn4-idempotent",
  "scripts": {
    "test": "vp test run",
    "prepare": "vp config"
  },
  "devDependencies": {
    "vite": "catalog:",
    "vite-plus": "catalog:"
  },
  "packageManager": "yarn@4.12.0",
  "resolutions": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>"
  }
}
```

## `vpt print-file .yarnrc.yml`

managed catalog entries are available to those specs

```
nodeLinker: node-modules
npmPreapprovedPackages:
  - vitest
  - '@vitest/*'
catalog:
  vite: npm:@voidzero-dev/vite-plus-core@<version>
  vite-plus: <version>
```

## `vpt print-file example.spec.ts`

ordinary Vitest imports use the Vite+ public surface

```
import { expect, it } from 'vite-plus/test';

it('works', () => expect(true).toBe(true));
```

## `vp migrate --no-interactive`

a freshly migrated standalone Yarn project is complete

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
