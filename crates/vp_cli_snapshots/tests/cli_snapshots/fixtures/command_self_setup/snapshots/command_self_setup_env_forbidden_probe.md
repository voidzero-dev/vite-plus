# command_self_setup_env_forbidden_probe

## `vpt rm $VP_HOME/current/bin/.vp-setup-complete`


## `VP_SELF_SETUP_SUPPORT_CHECK=env-forbidden vp --help`

The env-forbidden check value adds a second line after the unchanged v1 response

```
vite-plus-self-setup-v1
vite-plus-self-setup-env-forbidden
```

## `VP_SELF_SETUP_SUPPORT_CHECK=Env-Forbidden vp --help`

Only the exact lowercase value asks for VP_ENV_FORBIDDEN support

```
vite-plus-self-setup-v1
```

## `vpt stat-file $VP_HOME/current/bin/.vp-setup-complete --assert missing`

```
<home>/.vite-plus/current/bin/.vp-setup-complete: missing
```
