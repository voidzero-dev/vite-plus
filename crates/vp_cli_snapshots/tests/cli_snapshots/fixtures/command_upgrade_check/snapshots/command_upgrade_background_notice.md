# command_upgrade_background_notice

A foreground command launches a detached update check without waiting for it, then later commands show the cached notice at most once per prompt interval.

## `vp env list`

The foreground command launches the detached checker and returns without waiting for registry I/O.


## `node -e '(async()=>{const fs=require('\''node:fs'\'');const path=require('\''node:path'\'');const file=path.join(process.env.VP_HOME,'\''cache'\'','\''upgrade-check.json'\'');const deadline=Date.now()+5000;for(;;){try{if(JSON.parse(fs.readFileSync(file,'\''utf8'\'')).status==='\''available'\'')return}catch{}if(Date.now()>=deadline)process.exit(1);await new Promise(resolve=>setTimeout(resolve,25))}})()'`


## `vpt grep-file $VP_HOME/cache/upgrade-check.json '"status":"available"'`


## `vp env list --json`

Machine-readable output does not consume the pending notice.


## `vpt write-file package.json '{"name":"silent-add","private":true,"packageManager":"npm@11.13.0"}'`


## `vpt mkdir dep`


## `vpt write-file dep/package.json '{"name":"silent-add-dep","version":"1.0.0"}'`


## `vp add ./dep --silent --save-catalog --lockfile-only`

Silent add reports unsupported options without consuming the cached update notice.

**Exit code:** 1

```
npm does not support --save-catalog.
```

## `vpt stat-file package-lock.json --assert missing`

```
package-lock.json: missing
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vp add ./dep --silent --lockfile-only`

Supported silent add succeeds without consuming the cached update notice.

```
```

## `vpt stat-file package-lock.json --assert file`


## `vp env off`

The next interactive command displays the cached update notice.

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.

vp update available: <version> → 999.0.0, run vp upgrade
```

## `vp env off`

A subsequent command stays quiet after the notice timestamp is recorded.

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```
