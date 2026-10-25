# command_self_setup_system_only_probe

## `vpt rm $VP_HOME/current/bin/.vp-setup-complete`


## `VP_SELF_SETUP_SUPPORT_CHECK=system-only vp --help`

The system-only check value adds a second line after the unchanged v1 response

```
vite-plus-self-setup-v1
vite-plus-self-setup-system-only
```

## `VP_SELF_SETUP_SUPPORT_CHECK=System-Only vp --help`

Only the exact lowercase value asks for system-only support

```
vite-plus-self-setup-v1
```

## `vpt stat-file $VP_HOME/current/bin/.vp-setup-complete --assert missing`

```
<home>/.vite-plus/current/bin/.vp-setup-complete: missing
```
