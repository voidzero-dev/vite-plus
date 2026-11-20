# command_dev_local_signal_reraised_sigint

Interactive Vite restores the terminal when a plugin removes its last SIGINT listener and re-sends SIGINT.

## `node probe.mjs --reraise-sigint`

```
Vite served the project with the client under /probe/
Vite printed its config debug logs
Vite ran the plugin callback and re-sent SIGINT
SIGINT to the launch PID closed the port and restored the terminal
```
