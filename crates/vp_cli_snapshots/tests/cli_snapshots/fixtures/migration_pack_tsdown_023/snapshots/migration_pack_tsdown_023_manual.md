# migration_pack_tsdown_023_manual

Report unsupported external matchers without changing the pack options.

## `vpt cp manual.config.txt vite.config.ts`


## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite-plus  0.2.0 → <version>
    vite             → <version>
• Package manager settings configured
! Warnings:
  - vite.config.ts: Cannot safely combine external with skipNodeModulesBundle. Migrate this pack config manually; its options were left unchanged.
```

## `vpt print-file vite.config.ts`

```
const externalOptions = ['foo'];

export default {
  pack: [
    { external: externalOptions, skipNodeModulesBundle: true, bundle: false, dts: { tsgo: true } },
    { external: externalOptions, deps: { skipNodeModulesBundle: true }, bundle: false },
  ],
};
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

◇ Updated . to Vite+ <version>
• Node <version>  npm <version>
• Dependencies:
    vite   → <version>
! Warnings:
  - vite.config.ts: Cannot safely combine external with skipNodeModulesBundle. Migrate this pack config manually; its options were left unchanged.
```

## `vpt print-file vite.config.ts`

```
const externalOptions = ['foo'];

export default {
  pack: [
    { external: externalOptions, skipNodeModulesBundle: true, bundle: false, dts: { tsgo: true } },
    { external: externalOptions, deps: { skipNodeModulesBundle: true }, bundle: false },
  ],
};
```
