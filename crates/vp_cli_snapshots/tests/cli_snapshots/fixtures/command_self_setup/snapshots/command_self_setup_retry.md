# command_self_setup_retry

## `vpt rm $VP_HOME/current/bin/.vp-setup-complete $VP_HOME/env`


## `vpt mkdir $VP_HOME/env`


## `vp env setup --refresh`

A failed setup does not mark the deployed binary as complete

**Exit code:** 1

```
error: Command execution failed: Is a directory (os error 21)
```

## `vpt stat-file $VP_HOME/current/bin/.vp-setup-complete --assert missing`

```
<home>/.vite-plus/current/bin/.vp-setup-complete: missing
```

## `vpt rm -r $VP_HOME/env`


## `vp env setup --refresh`

The same upgrade handoff retries setup after the failure is repaired


## `vpt stat-file $VP_HOME/current/bin/.vp-setup-complete --assert file`

```
<home>/.vite-plus/current/bin/.vp-setup-complete: file
```
