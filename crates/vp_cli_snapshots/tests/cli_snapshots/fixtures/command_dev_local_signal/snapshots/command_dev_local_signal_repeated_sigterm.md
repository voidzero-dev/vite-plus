# command_dev_local_signal_repeated_sigterm

Interactive Vite restores the terminal when a second SIGTERM interrupts asynchronous shutdown.

## `node probe.mjs --repeat-sigterm`

```
Vite served the project with the client under /probe/
Vite began asynchronous shutdown after the first SIGTERM
Vite printed its config debug logs
SIGTERM to the launch PID closed the port and restored the terminal
```
