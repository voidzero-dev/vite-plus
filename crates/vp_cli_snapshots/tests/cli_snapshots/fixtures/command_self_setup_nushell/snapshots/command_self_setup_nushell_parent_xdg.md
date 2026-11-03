# command_self_setup_nushell_parent_xdg

Setting XDG_DATA_HOME before Nushell starts lets new sessions load the installed snippet.

## `vpt mkdir external`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `nu --no-config-file assert.nu parent`

```
Standalone setup completed and wrote vite-plus.nu
Both sessions have the configured XDG_DATA_HOME after startup: true
Installer and session directories match: true
Fresh session loaded Vite+ environment: true
```
