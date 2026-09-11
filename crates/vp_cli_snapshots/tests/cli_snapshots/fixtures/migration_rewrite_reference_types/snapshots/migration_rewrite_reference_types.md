# migration_rewrite_reference_types

## `vp migrate --no-interactive`

vitest/tsdown reference types are rewritten; vite references are preserved (issue #2004)

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt print-file src/env.d.ts`

vite references stay on vite (non-config); vitest/tsdown references become vite-plus

```
/// <reference types="vite" />
/// <reference types="vite/client" />
/// <reference types="vite-plus/test" />
/// <reference types="vite-plus/test/globals" />
/// <reference types="vite-plus" />
/// <reference types="vite-plus/test/browser/context" />
/// <reference types="vite-plus/pack/client" />
```
