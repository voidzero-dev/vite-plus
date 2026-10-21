# vpx_script_ctrl_c_cleanup

Ctrl+C reaches the script, and vpx exits after the script's SIGINT cleanup, with its exit code.

## `vpx ./ctrlc.ts`

**→ expect-milestone:** `ready`

```
```

**← write-key:** `ctrl-c`

```
SIGINT: cleaning up
cleaned up
```
