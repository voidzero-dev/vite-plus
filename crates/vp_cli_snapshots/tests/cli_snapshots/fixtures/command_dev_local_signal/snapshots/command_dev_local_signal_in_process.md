# command_dev_local_signal_in_process

Node.js options, a preload, --debug, and an IPC channel of the launcher apply once, to Vite in the launcher process.

## `node probe-in-process.mjs`

```
node --cpu-prof: applied once, to Vite in the launch PID
NODE_OPTIONS=--cpu-prof: applied once, to Vite in the launch PID
NODE_OPTIONS=--inspect: applied once, to Vite in the launch PID
NODE_V8_COVERAGE: applied once, to Vite in the launch PID
NODE_OPTIONS=--import: applied once, to Vite in the launch PID
vp dev --debug config: applied once, to Vite in the launch PID
fork() with IPC: applied once, to Vite in the launch PID
```
