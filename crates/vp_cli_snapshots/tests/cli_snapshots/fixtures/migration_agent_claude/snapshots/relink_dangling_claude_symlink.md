# relink_dangling_claude_symlink

When both agent files are selected, migration repairs a dangling CLAUDE.md link.

## `node -e 'require('\''node:fs'\'').symlinkSync('\''missing.md'\'', '\''CLAUDE.md'\'')'`


## `vp migrate --agent agents,claude --no-interactive --no-hooks --no-editor`

```
VITE+ - The Unified Toolchain for the Web

◇ Migrated . to Vite+ <version>
• Node <version>  pnpm <version>
• 1 config update applied, 1 file had imports rewritten
```

## `node -p require('node:fs').readlinkSync('CLAUDE.md')`

```
AGENTS.md
```

## `vpt grep-file CLAUDE.md '<'\!'--VITE PLUS START-->'`

```
CLAUDE.md: found "<!--VITE PLUS START-->"
```
