# command_upgrade_rollback_deployed

A rollback holds the installation lock while an unmarked target completes in-place setup, so the child must not take that lock.

## `vpt mkdir external`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `VP_HOME=${workspace}/home VP_VERSION=rollback-a ./external/vp`

Deploy the first version


## `VP_HOME=${workspace}/home VP_VERSION=rollback-b ./external/vp`

Deploy the second version, which records the first as the rollback target


## `vpt print-file home/.previous-version`

```
rollback-a
```

## `vpt rm home/rollback-a/bin/.vp-setup-complete`

The rollback target must run in-place setup during the shim refresh


## `VP_HOME=${workspace}/home vp upgrade --rollback`

A deadlock fails this step by timeout


## `vpt print-file home/.previous-version`

```
rollback-b
```

## `vpt stat-file home/rollback-a/bin/.vp-setup-complete --assert file`

```
home/rollback-a/bin/.vp-setup-complete: file
```
