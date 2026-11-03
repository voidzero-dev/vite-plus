# command_self_setup_nushell_config_xdg_mismatch

Issue #2491: with XDG_DATA_HOME unset in the parent and set in config.nu, installation succeeds but a fresh session does not load Vite+.

## `vpt mkdir external`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `nu --no-config-file assert.nu config`

```
Standalone setup completed and wrote vite-plus.nu
Both sessions have the configured XDG_DATA_HOME after startup: true
Installer and session directories match: false
Fresh session loaded Vite+ environment: false
After adding source to config.nu, a fresh session loaded Vite+: true
vp help succeeded in the fresh session
```
