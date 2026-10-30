# command_env_list_remote

## `vp env default node`

An unconfigured Node.js default explains the effective fallback without claiming it was set in config

```
VITE+ - The Unified Toolchain for the Web

No default Node.js version configured. Using latest LTS (<version>).
  Run 'vp env default <version>' to set a default.
```

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

✓ Default Node.js version set to lts (currently <version>)
```

## `vp env default node`

A configured Node.js alias shows its current resolution and config source

```
VITE+ - The Unified Toolchain for the Web

Default Node.js version: lts
  Currently resolves to: <version>
  Set via: <home>/.vite-plus/config.json
```

## `vp env default pnpm@10.18.0`

Package-manager default updates identify the selected family and version

```
VITE+ - The Unified Toolchain for the Web

✓ Default pnpm version set to 10.18.0
```

## `node -e 'const {execFileSync}=require('\''node:child_process'\''); const {node}=JSON.parse(execFileSync('\''vp'\'',['\''env'\'','\''list-remote'\'','\''--lts'\'','\''--json'\''],{encoding:'\''utf8'\''})); console.log('\''installed marked:'\'', node.some(v=>v.installed)); console.log('\''current marked:'\'', node.some(v=>v.current)); console.log('\''default marked:'\'', node.some(v=>v.default));'`

the unified JSON node entries resolve installed/current/default flags, including the `lts` default alias

```
installed marked: true
current marked: true
default marked: true
```

## `vp env list-remote node 22.11.0`

Human-readable Node.js results keep the v prefix, LTS codename, and interactive formatting

```
VITE+ - The Unified Toolchain for the Web

Node.js
  <version>\x1b[94m (Jod)

\x1b[2mnote: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```

## `vp env list-remote pnpm 10.18.0`

Human-readable package-manager results retain current-version formatting

```
VITE+ - The Unified Toolchain for the Web

pnpm
  \x1b[94m10.18.0\x1b[39;2m current default

\x1b[2mnote: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```

## `vp env list-remote node 999`

An empty Node.js result includes actionable feedback

```
VITE+ - The Unified Toolchain for the Web

Node.js
  No versions were found!

note: Run `vp env clean` to free disk space from unused managed runtimes and package manager caches.
```
