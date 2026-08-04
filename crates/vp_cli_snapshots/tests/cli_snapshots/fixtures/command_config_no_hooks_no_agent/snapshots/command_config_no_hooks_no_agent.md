# command_config_no_hooks_no_agent

## `git init`


## `vp config --no-hooks --no-agent`

should skip hook installation and agent instruction updates

```
```

## `vpt stat-file .vite-hooks/_/pre-commit --assert missing`

should not install hooks

```
.vite-hooks/_/pre-commit: missing
```

## `vpt grep-file AGENTS.md 'OUTDATED CONTENT'`

agent must stay unchanged (outdated marker still present)

```
AGENTS.md: found "OUTDATED CONTENT"
```
