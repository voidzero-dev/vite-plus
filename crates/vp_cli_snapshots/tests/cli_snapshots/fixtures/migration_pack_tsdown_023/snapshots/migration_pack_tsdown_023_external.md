# migration_pack_tsdown_023_external

Preserve external matchers when either skipNodeModulesBundle form becomes deps.neverBundle.

## `vpt cp external.config.txt vite.config.ts`


## `vpt cp external-entry.txt src/index.ts`


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
export default {
  pack: [
    {
      entry: 'src/index.ts',
      outDir: 'dist/top',
      dts: false,
      inputOptions: { external: ['foo', './external.js'] },
      deps: { neverBundle: true, resolveDepSubpath: true },
    },
    {
      entry: 'src/index.ts',
      outDir: 'dist/nested',
      dts: false,
      inputOptions: { external: ['foo', './external.js'] },
      deps: { resolveDepSubpath: true, neverBundle: true },
    },
  ],
};
```

## `vp pack`


## `vpt print-file dist/top/index.mjs`

```
import { foo } from "foo";
import { externalValue } from "./external.js";
export { externalValue, foo };
```

## `vpt print-file dist/nested/index.mjs`

```
import { foo } from "foo";
import { externalValue } from "./external.js";
export { externalValue, foo };
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```
