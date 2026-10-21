# vpx_script_ipc

An IPC channel from the process that starts vpx reaches the script in both directions, including messages sent right before it exits.

## `node parent.mjs`

```
reply {"echo":"ping"}
messages before exit 100 exit code 3
exit code after the parent disconnects 4
```
