# migration_pack_tsdown_023_no_external

Migrate noExternal patterns, references, and callbacks and confirm the dependency remains bundled.

## `vpt cp no-external.config.txt vite.config.ts`


## `vpt cp no-external-entry.txt src/index.ts`


## `vpt json-edit package.json dependencies '{"@fixture/pack-bundled":"file:./pack-dependency"}'`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  0.2.0 → <version>
    vite             → <version>
• 1 file had imports rewritten
• Package manager settings configured
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';

const bundlePatterns = ['@fixture/pack-bundled'];

export default defineConfig({
  pack: [
    {
      entry: 'src/index.ts', outDir: 'dist/literal', dts: false,
      deps: { alwaysBundle: ['@fixture/pack-bundled'], resolveDepSubpath: true },
    },
    {
      entry: 'src/index.ts', outDir: 'dist/reference', dts: false,
      deps: { alwaysBundle: bundlePatterns, resolveDepSubpath: true },
    },
    {
      entry: 'src/index.ts', outDir: 'dist/callback', dts: false,
      deps: { alwaysBundle: (id) => bundlePatterns.includes(id), resolveDepSubpath: true },
    },
    {
      entry: 'src/index.ts', outDir: 'dist/method', dts: false,

      deps: { alwaysBundle(id) { return bundlePatterns.includes(id); }, resolveDepSubpath: true, onlyBundle: bundlePatterns },
    },
    { deps: { resolveDepSubpath: true },
      entry: 'src/index.ts', outDir: 'dist/control', dts: false,
    },
  ],
});
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

```
import { defineConfig } from 'vite-plus';

const bundlePatterns = ['@fixture/pack-bundled'];

export default defineConfig({
  pack: [
    {
      entry: 'src/index.ts', outDir: 'dist/literal', dts: false,
      deps: { alwaysBundle: ['@fixture/pack-bundled'], resolveDepSubpath: true },
    },
    {
      entry: 'src/index.ts', outDir: 'dist/reference', dts: false,
      deps: { alwaysBundle: bundlePatterns, resolveDepSubpath: true },
    },
    {
      entry: 'src/index.ts', outDir: 'dist/callback', dts: false,
      deps: { alwaysBundle: (id) => bundlePatterns.includes(id), resolveDepSubpath: true },
    },
    {
      entry: 'src/index.ts', outDir: 'dist/method', dts: false,

      deps: { alwaysBundle(id) { return bundlePatterns.includes(id); }, resolveDepSubpath: true, onlyBundle: bundlePatterns },
    },
    { deps: { resolveDepSubpath: true },
      entry: 'src/index.ts', outDir: 'dist/control', dts: false,
    },
  ],
});
```

## `vp install`


## `vpt stat-file node_modules/@fixture/pack-bundled/index.js --assert file`

```
node_modules/@fixture/pack-bundled/index.js: file
```

## `vp pack --fail-on-warn`


## `vpt print-file dist/literal/index.mjs`

```
//#region pack-dependency/index.js
const bundledValue = 42;
//#endregion
export { bundledValue };
```

## `vpt print-file dist/reference/index.mjs`

```
//#region pack-dependency/index.js
const bundledValue = 42;
//#endregion
export { bundledValue };
```

## `vpt print-file dist/callback/index.mjs`

```
//#region pack-dependency/index.js
const bundledValue = 42;
//#endregion
export { bundledValue };
```

## `vpt print-file dist/method/index.mjs`

```
//#region pack-dependency/index.js
const bundledValue = 42;
//#endregion
export { bundledValue };
```

## `vpt print-file dist/control/index.mjs`

```
import { bundledValue } from "@fixture/pack-bundled";
export { bundledValue };
```
