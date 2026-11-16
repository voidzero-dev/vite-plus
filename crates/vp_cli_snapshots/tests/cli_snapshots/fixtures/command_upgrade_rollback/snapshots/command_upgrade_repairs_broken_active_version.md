# command_upgrade_repairs_broken_active_version

An upgrade to the version that `current` names still installs it when the active binary is missing.

## `vpt mkdir external`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `VP_HOME=${workspace}/home VP_VERSION=999.0.1 ./external/vp`

Activate a 999.0.1 directory


## `vpt rm -r home/999.0.1/bin`

Break the active installation


## `VP_HOME=${workspace}/home vp upgrade 999.0.1 --silent`

**Exit code:** 1

```
error: Upgrade error: Binary not found after extraction. The download may be corrupted.
```
