# command_env_unset_session_independently_of_override

Scoped unset must inspect and clear the session file independently of any different environment override.

## `vp env use pnpm@10.18.0 --no-install`


## `VP_PACKAGE_MANAGER=yarn@4.12.0 vp env use --unset pnpm`


## `vpt stat-file $VP_HOME/.session-package-manager --assert missing`

a different environment override does not hide the matching session file from scoped cleanup

```
<home>/.vite-plus/.session-package-manager: missing
```
