# migration_pack_tsdown_023_external_conflicts

Report conflicting dependency rules while preserving the pack options.

## `vpt cp external-conflicts.config.txt vite.config.ts`


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
  - vite.config.ts: Cannot safely combine external with skipNodeModulesBundle. Migrate this pack config manually; its options were left unchanged. See https://tsdown.dev/options/dependencies#migration-from-deprecated-options
```

## `vpt print-file vite.config.ts`

```
const externalOptions = ['foo'];

export default {
  pack: [
    {
      external: externalOptions,
      skipNodeModulesBundle: true,
      inputOptions(options) { return { external: options.external }; },
      bundle: false,
      dts: { tsgo: true },
    },
    {
      external: externalOptions,
      deps: { skipNodeModulesBundle: true, dts: { neverBundle: ['types'] } },
      bundle: false,
    },
  ],
};
```
