# migration_framework_shim_astro

## `vp migrate --no-interactive --no-hooks`

migration should add Astro shim when astro dependency is detected

```
VITE+ - The Unified Toolchain for the Web

Formatting code...

Code formatted
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
✓ Dependencies installed in <duration>
• 1 config update applied
• TypeScript shim added for framework component files
```

## `vpt print-file src/env.d.ts`

check Astro shim was written

```
/// <reference types="astro/client" />
```
