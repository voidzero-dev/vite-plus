# migration_agent_claude

## `vp migrate --agent claude --no-interactive`

migration with --agent claude should write CLAUDE.md

```
VITE+ - The Unified Toolchain for the Web

Vitest v5: 1 review item

package.json
  1:1 REVIEW [configless-defaults] No test config exists. Vitest v5 clears mocks by default. A separate confirmed action can create compatibility config, but a new config can change config discovery and project structure.
◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt grep-file CLAUDE.md '<'\!'--VITE PLUS START-->'`

CLAUDE.md was created with the Vite+ agent block

```
CLAUDE.md: found "<!--VITE PLUS START-->"
```
