# command_config_no_hooks

## `git init`


## `vp config --no-hooks`

should skip hook installation but update agent instructions

```
```

## `vpt stat-file .vite-hooks/_/pre-commit --assert missing`

should not install hooks

```
.vite-hooks/_/pre-commit: missing
```

## `vpt grep-file AGENTS.md 'OUTDATED CONTENT'`

agent updated: the outdated marker must be gone (grep-file prints missing)

**Exit code:** 1

```
AGENTS.md: missing "OUTDATED CONTENT"
pattern not found
```
