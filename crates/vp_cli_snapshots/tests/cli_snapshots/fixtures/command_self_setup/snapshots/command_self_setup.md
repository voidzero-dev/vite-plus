# command_self_setup

## `vpt rm $VP_HOME/current/bin/.vp-setup-complete`


## `VP_SELF_SETUP_SUPPORT_CHECK=1 vp --help`

The capability probe does not perform setup

```
vite-plus-self-setup-v1
```

## `vpt stat-file $VP_HOME/current/bin/.vp-setup-complete --assert missing`

```
<home>/.vite-plus/current/bin/.vp-setup-complete: missing
```

## `vp not-a-command`

An unmarked deployed binary consumes the invocation as setup, without parsing the command


## `vpt stat-file $VP_HOME/current/bin/.vp-setup-complete --assert file`

```
<home>/.vite-plus/current/bin/.vp-setup-complete: file
```

## `vp not-a-command`

Once marked, the binary dispatches commands normally

**Exit code:** 2

```
VITE+ - The Unified Toolchain for the Web

error: Command 'not-a-command' not found
```
