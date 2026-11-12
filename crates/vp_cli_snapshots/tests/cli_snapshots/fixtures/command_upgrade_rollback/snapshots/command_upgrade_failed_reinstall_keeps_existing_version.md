# command_upgrade_failed_reinstall_keeps_existing_version

A failed upgrade into an existing version directory keeps that directory, which can be the rollback target.

## `vpt write-file $VP_HOME/999.0.1/keep.txt 'rollback target
'`


## `vpt write-file $VP_HOME/999.0.1/bin 'not a directory
'`

Make extraction fail after the download is verified


## `vp upgrade 999.0.1 --silent`

**Exit code:** 1

```
error: I/O error: File exists (os error 17)
```

## `vpt print-file $VP_HOME/999.0.1/keep.txt`

```
rollback target
```
