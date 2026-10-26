# command_config_first_run_with_config_lock

## `git init`


## `vpt touch-file .git/config.lock`

a concurrent run in a linked worktree holds the shared config lock and does not release it


## `vp config --no-agent`

first run retries the lock, then reports the Git error

**Exit code:** 1

```
error: could not lock config file .git/config: File exists
```

## `vpt rm .git/config.lock`

the other run releases the lock


## `vp config --no-agent`

next run writes the git config

```
```

## `git config --local core.hooksPath`

should be .vite-hooks/_

```
.vite-hooks/_
```
