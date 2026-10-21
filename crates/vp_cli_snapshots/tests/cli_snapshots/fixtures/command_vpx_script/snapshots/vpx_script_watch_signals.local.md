# vpx_script_watch_signals

A SIGINT sent to the vpx --watch PID reaches the script once, and so does a SIGINT to its whole process group, as Ctrl+C sends it.

## `node watch.mjs`

```
pid: the script runs in a child of vpx: true
pid: SIGINT 1
pid: exit code 0, signal null
group: the script runs in a child of vpx: true
group: SIGINT 1
group: exit code 0, signal null
```
