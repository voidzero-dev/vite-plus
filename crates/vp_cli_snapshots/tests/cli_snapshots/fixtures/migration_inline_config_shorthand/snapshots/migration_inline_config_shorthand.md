# migration_inline_config_shorthand

## `vp migrate --no-interactive --no-hooks`

must NOT duplicate fmt/lint already declared as shorthand properties (#1836)

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
```

## `vpt print-file vite.config.ts`

fmt/lint stay as shorthand only, no injected inline fmt:/lint: blocks

```
import { defineConfig } from 'vite-plus';

// Mirrors a custom template that keeps tooling config in separate modules and
// wires them in with shorthand properties (`fmt,` / `lint,`). See #1836.
const fmt = { ignorePatterns: [] };
const lint = { rules: {} };

export default defineConfig(({ mode }) => {
  return {
    server: { port: 3000 },
    fmt,
    lint,
  };
});
```

## `vpt stat-file .oxlintrc.json --assert-not file`

no standalone lint config generated

```
.oxlintrc.json: missing
```

## `vpt stat-file .oxfmtrc.json --assert-not file`

no standalone fmt config generated

```
.oxfmtrc.json: missing
```
