# command_env_node18

## `vp env install 18.20.8`

Install the oldest supported Node.js release

```
VITE+ - The Unified Toolchain for the Web

Installing Node.js <version>...
Installed Node.js <version>
```

## `vp env exec --node 18.20.8 node -p ''\''Node major: '\'' + process.versions.node.split('\''.'\'')[0]'`

Run Node.js 18 through vp env exec

```
Node major: 18
```

## `vpt write-file .node-version '18.20.8
'`


## `node -p ''\''Node major: '\'' + process.versions.node.split('\''.'\'')[0]'`

The node shim selects the project's Node.js 18

```
Node major: 18
```
