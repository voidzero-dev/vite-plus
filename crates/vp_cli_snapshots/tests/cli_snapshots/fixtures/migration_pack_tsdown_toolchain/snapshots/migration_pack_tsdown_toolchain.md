# migration_pack_tsdown_toolchain

Use the original config imports to distinguish legacy vp pack from standalone tsdown 0.23.0 in the same workspace, with both tools installed at the root.

## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.2.0"}'`


## `vpt write-file node_modules/vite/package.json '{"name":"@voidzero-dev/vite-plus-core","version":"0.2.0","bundledVersions":{"vite":"8.0.0"}}'`


## `vpt write-file node_modules/tsdown/package.json '{"name":"tsdown","version":"0.23.0"}'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  pnpm <version>
• Dependencies:
    vite-plus  0.2.0 → <version>
    vite       8.0.0 → <version>
• 2 config updates applied, 2 files had imports rewritten
• Package manager settings configured
→ Manual follow-up:
  - Please manually merge packages/standalone/tsdown.config.ts into packages/standalone/vite.config.ts, see https://viteplus.dev/guide/migrate#tsdown
```

## `vpt print-file tsdown.config.ts`

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({ deps: {
    // tsdown <0.23 compatibility: resolve external dependency subpaths.
    // Remove to preserve subpath imports as written (the new default).
    // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
    resolveDepSubpath: true },
  entry: ['src/index.ts'],
  dts: true,
  attw: {
    // tsdown <0.23 compatibility: keep all declaration resolution checks.
    // Remove to use the new 'esm-only' profile.
    // https://tsdown.dev/options/lint#profiles
    profile: 'strict' },
});
```

## `vpt print-file vite.config.ts`

```
import config from './tsdown.config.js';
import { defineConfig } from 'vite-plus';

export default defineConfig({ pack: config });
```

## `vpt print-file packages/standalone/tsdown.config.ts`

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  attw: true,
});
```

## `vpt print-file packages/standalone/vite.config.ts`

```
import tsdownConfig from './tsdown.config.js';

import { defineConfig } from 'vite-plus';

export default defineConfig({
  pack: tsdownConfig,

});
```

## `vpt print-file packages/standalone/package.json`

```
{
  "name": "standalone-tsdown-library",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "vp pack"
  },
  "devDependencies": {}
}
```

## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"1.0.1"}'`


## `vpt write-file node_modules/vite-plus/dist/toolchain.json '{"schemaVersion":1,"nodes":[{"id":"tsdown","name":"tsdown","version":"0.23.0","kind":"tool","delivery":["bundled"]}],"edges":[]}'`

Model the final install before repeating migration.


## `vpt rm -r node_modules/tsdown`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file tsdown.config.ts`

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({ deps: {
    // tsdown <0.23 compatibility: resolve external dependency subpaths.
    // Remove to preserve subpath imports as written (the new default).
    // https://tsdown.dev/options/dependencies#deps-resolvedepsubpath
    resolveDepSubpath: true },
  entry: ['src/index.ts'],
  dts: true,
  attw: {
    // tsdown <0.23 compatibility: keep all declaration resolution checks.
    // Remove to use the new 'esm-only' profile.
    // https://tsdown.dev/options/lint#profiles
    profile: 'strict' },
});
```

## `vpt print-file packages/standalone/tsdown.config.ts`

```
import { defineConfig } from 'vite-plus/pack';

export default defineConfig({
  entry: ['src/index.ts'],
  dts: true,
  attw: true,
});
```
