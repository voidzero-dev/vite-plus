# command_dev_local_signal_diagnostics_env

NODE_OPTIONS=--inspect, NODE_OPTIONS=--cpu-prof, and NODE_V8_COVERAGE keep Vite as a child of the launcher.

## `node probe-diagnostics.mjs`

```
NODE_OPTIONS=--inspect kept Vite as a child process
NODE_OPTIONS=--cpu-prof kept Vite as a child process, and the launcher wrote its output
NODE_V8_COVERAGE kept Vite as a child process, and the launcher wrote its output
```
