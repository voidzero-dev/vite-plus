# command_self_setup_handoff

## `vpt mkdir -p external project home/js_runtime/node`


## `vpt cp -r $VP_HOME/js_runtime/node/22.18.0 home/js_runtime/node/22.18.0`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `VP_HOME=${workspace}/home VP_SKIP_DEPS_INSTALL=1 VP_VERSION=handoff-test VP_NODE_MANAGER=no VP_SELF_SETUP_NO_MODIFY_PATH=1 HANDOFF_VALUE=original environment vpt pipe-stdin 'original stdin' -- bash -c 'exec ./external/vp "$@" 2>setup.log' -- -C project env exec --node 22.18.0 node -e 'console.log(JSON.stringify({cwd: require('\''node:path'\'').basename(process.cwd()), value: process.env.HANDOFF_VALUE, input: require('\''node:fs'\'').readFileSync(0, '\''utf8'\'')})); process.exit(17)'`

The deployed binary executes the original command with its cwd, environment, stdin and exit status

**Exit code:** 17

```
{"cwd":"project","value":"original environment","input":"original stdin\n"}
```
