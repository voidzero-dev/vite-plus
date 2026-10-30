# upgrade_node_metadata_failure

## `node upgrade-node.mjs failure`

```
Upgrade exit: 1
stderr: error: Setup error: Failed to install production dependencies (exit code: 17). See log for details: <upgrade-home>/upgrade.log
Install attempts: 1
Attempt 1: runtime=22.23.4; expected executable=true
  During install: current is original=true; rollback=rollback-sentinel
  Install inputs: frozen=true; lockfile matches=true; dependency matches=true
  After upgrade: staged exists=false; receipt matches=false; current is staged=false
Current is original: true
Rollback: rollback-sentinel
Remaining staged directories: 0
Requests:
  /vite-plus/latest
  /@voidzero-dev/vite-plus-cli-<platform>/<release>
  /platform.tgz
```
