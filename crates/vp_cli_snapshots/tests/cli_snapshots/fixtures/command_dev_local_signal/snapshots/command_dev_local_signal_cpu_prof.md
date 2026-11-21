# command_dev_local_signal_cpu_prof

A launcher started with a Node.js flag such as --cpu-prof keeps Vite as a child and writes its profile.

## `node probe.mjs --cpu-prof`

```
Vite served the project with the client under /probe/
The profiled launcher kept Vite as a child process
SIGTERM to the Vite child closed the port with exit code 143
The launcher wrote its CPU profile
```
