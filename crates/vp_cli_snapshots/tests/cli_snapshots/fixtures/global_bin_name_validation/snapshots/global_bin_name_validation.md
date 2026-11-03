# global_bin_name_validation

Skip unsafe bin names and targets even with --ignore-scripts, while installing valid commands and preserving files outside VP_HOME.

## `node verify.mjs`

```
Mixed update installs safe and scoped commands, and skips unsafe names and targets.
Invalid-only update installs without commands; uninstall preserves outside files.
```
