# migration_no_agent

## `vp migrate --no-agent --no-interactive`

migration with --no-agent should skip agent instructions

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 2 config updates applied, 1 file had imports rewritten
```

## `vpt stat-file AGENTS.md --assert-not file`

no AGENTS.md created with --no-agent

```
AGENTS.md: missing
```

## `vpt stat-file CLAUDE.md --assert-not file`

no CLAUDE.md created with --no-agent

```
CLAUDE.md: missing
```
