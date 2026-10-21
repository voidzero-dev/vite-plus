# vp_run_vpx_script_ctrl_c_cleanup

Ctrl+C under vp run reaches the script that vpx runs, and vp run exits after the script's SIGINT cleanup.

## `vp run cleanup`

**→ expect-milestone:** `ready`

```
VITE+ - The Unified Toolchain for the Web

$ vpx ./ctrlc.ts ⊘ cache disabled
```

**← write-key:** `ctrl-c`

```
VITE+ - The Unified Toolchain for the Web

$ vpx ./ctrlc.ts ⊘ cache disabled
SIGINT: cleaning up
cleaned up
```
