# command_env_list_remote

## `vp env install lts`

Install an LTS Node.js version locally

```
VITE+ - The Unified Toolchain for the Web

Installing Node.js <version>...
Installed Node.js <version>
```

## `vp env default lts`

Set it as the global default (stored as the `lts` alias)

```
VITE+ - The Unified Toolchain for the Web

✓ Environment defaults updated.
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\''); const {node}=JSON.parse(execFileSync('\''vp'\'',['\''env'\'','\''list-remote'\'','\''--lts'\'','\''--json'\''],{encoding:'\''utf8'\''})); console.log('\''installed marked:'\'', node.some(v=>v.installed)); console.log('\''current marked:'\'', node.some(v=>v.current)); console.log('\''default marked:'\'', node.some(v=>v.default));'`

the unified JSON node entries resolve installed/current/default flags, including the `lts` default alias

```
installed marked: true
current marked: true
default marked: true
```
