# migration_pack_tsdown_023_external_constants

Migrate constant external references with both skip forms without a manual-migration warning.

## `vpt cp external-constants.config.txt vite.config.ts`


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
const externalOptions = ['foo'];

export default {
  pack: [
    { inputOptions: { external: externalOptions }, deps: { neverBundle: true, resolveDepSubpath: true }, unbundle: true, dts: { generator: 'tsgo' } },
    { inputOptions: { external: externalOptions }, deps: { resolveDepSubpath: true, neverBundle: true }, unbundle: true },
  ],
};
```

## `vp migrate --no-interactive`

```
VITE+ - The Unified Toolchain for the Web

This project is already using Vite+! Happy coding!
```

## `vpt print-file vite.config.ts`

```
const externalOptions = ['foo'];

export default {
  pack: [
    { inputOptions: { external: externalOptions }, deps: { neverBundle: true, resolveDepSubpath: true }, unbundle: true, dts: { generator: 'tsgo' } },
    { inputOptions: { external: externalOptions }, deps: { resolveDepSubpath: true, neverBundle: true }, unbundle: true },
  ],
};
```
