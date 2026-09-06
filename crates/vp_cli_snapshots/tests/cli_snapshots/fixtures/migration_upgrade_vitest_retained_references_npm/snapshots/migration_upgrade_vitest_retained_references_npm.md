# migration_upgrade_vitest_retained_references_npm

## `vp migrate --no-interactive`

retained upstream references require package-local Vitest

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  latest → <version>
    vite              → <version>
    vitest     4.1.8  → <version>
• Package manager settings configured
! Warnings:
  - Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
```

## `vpt print-file package.json`

Vitest dependency and override stay aligned

```
{
  "name": "migration-upgrade-vitest-retained-references-npm",
  "devDependencies": {
    "vite-plus": "<version>",
    "vitest": "<version>"
  },
  "overrides": {
    "vite": "npm:@voidzero-dev/vite-plus-core@<version>",
    "vitest": "<version>"
  },
  "devEngines": {
    "packageManager": {
      "name": "npm",
      "version": "<version>",
      "onFail": "download"
    }
  }
}
```

## `vpt print-file tsconfig.json`

compilerOptions.types remains an upstream Vitest reference

```
{
  "compilerOptions": {
    "types": ["vitest/globals"]
  }
}
```

## `vpt print-file config/tsconfig.test.json`

nested compilerOptions.types is also retained

```
{
  "compilerOptions": {
    "types": ["vitest/globals"]
  }
}
```

## `vpt print-file resolve.cjs`

require.resolve remains an upstream Vitest reference

```
module.exports = require.resolve('vitest');
```

## `vpt print-file version.ts`

vitest/package.json remains intentionally unre-written

```
import metadata from 'vitest/package.json';

console.log(metadata.version);
```

## `vp migrate --no-interactive`

retained references remain stable on rerun

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
This project is already using Vite+! Happy coding!
```
