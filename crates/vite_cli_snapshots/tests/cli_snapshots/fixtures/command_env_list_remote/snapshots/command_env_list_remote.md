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

✓ Default Node.js version set to lts (currently 24.18.0)
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\''); const {versions}=JSON.parse(execFileSync('\''vp'\'',['\''env'\'','\''list-remote'\'','\''--lts'\'','\''--json'\''],{encoding:'\''utf8'\''})); console.log('\''installed marked:'\'', versions.some(v=>v.installed)); console.log('\''current marked:'\'', versions.some(v=>v.current)); console.log('\''default marked:'\'', versions.some(v=>v.default));'`

installed/current/default flags should all resolve, including the `lts` default alias

```
installed marked: true
current marked: true
default marked: true
```
