# task_overridden_by_plugin

A plugin config hook can override a statically analyzable task.

## `vpt replace-file-content vite.config.ts 'export default {' 'export default { run: { tasks: { '\''build:prod'\'': { command: '\''vpt exit 1'\'' } } },'`


## `vp run build:prod`

```
$ vpt print build:prod from plugin config hook
build:prod from plugin config hook
```
