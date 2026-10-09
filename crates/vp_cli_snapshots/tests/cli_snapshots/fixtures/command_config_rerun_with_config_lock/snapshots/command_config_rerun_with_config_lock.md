# command_config_rerun_with_config_lock

## `git init`


## `vp config --no-agent`

first run writes the git config

```
```

## `vpt touch-file .git/config.lock`

a concurrent run in a linked worktree holds the shared config lock


## `vp config --no-agent`

unchanged config should not need the lock

```
```

## `git config --local core.hooksPath`

should be .vite-hooks/_

```
.vite-hooks/_
```
