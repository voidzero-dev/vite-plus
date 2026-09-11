# env_node_list_stays_offline_for_floating_default

Local Node.js inventory must not require mirror access merely to mark a floating default.

## `vpt write-file $VP_HOME/js_runtime/node/20.18.0/bin/node '#'\!'/bin/sh
'`


## `vpt write-file $VP_HOME/config.json '{"defaultNodeVersion":"latest"}
'`


## `VP_NODE_DIST_MIRROR=http://127.0.0.1:9 vp env list node --json`

local Node.js listing remains available when a floating default cannot reach its mirror

```
{
  "node": [
    {
      "version": "20.18.0",
      "current": false,
      "default": false
    }
  ]
}
```
