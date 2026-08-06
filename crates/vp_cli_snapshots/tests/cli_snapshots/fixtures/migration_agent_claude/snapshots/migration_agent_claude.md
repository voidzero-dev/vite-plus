# migration_agent_claude

## `vp migrate --agent claude --no-interactive`

migration with --agent claude should write CLAUDE.md

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt grep-file CLAUDE.md '<'\!'--VITE PLUS START-->'`

CLAUDE.md was created with the Vite+ agent block

```
CLAUDE.md: found "<!--VITE PLUS START-->"
```
