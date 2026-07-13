# command_config_update_agents

## `git init`


## `vp config`

should auto-update agent instructions

```
```

## `vpt grep-file AGENTS.md 'Custom instructions here.'`

user content above the managed block preserved

```
AGENTS.md: found "Custom instructions here."
```

## `vpt grep-file AGENTS.md 'More custom content below.'`

user content below the managed block preserved

```
AGENTS.md: found "More custom content below."
```

## `vpt grep-file AGENTS.md 'OUTDATED CONTENT'`

outdated content replaced (grep-file prints missing)

**Exit code:** 1

```
AGENTS.md: missing "OUTDATED CONTENT"
pattern not found
```
