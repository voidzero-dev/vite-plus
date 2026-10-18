# vpx_script_signals

A signal sent to the PID of vpx reaches the script, as tools that stop a TypeScript runner send it.

## `node parent.mjs`

```
./wait.ts: the script runs in the vpx process: true
./wait.ts: exit code null, signal SIGINT
./graceful.ts: the script runs in the vpx process: true
./graceful.ts: cleaned up after SIGINT
./graceful.ts: exit code 0, signal null
```
