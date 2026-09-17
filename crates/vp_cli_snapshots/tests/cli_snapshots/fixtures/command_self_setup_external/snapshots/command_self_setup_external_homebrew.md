# command_self_setup_external_homebrew

## `vpt mkdir -p brew-prefix/bin home`


## `vpt cp $VP_HOME/bin/vp brew-prefix/bin/vp`


## `vpt write-file brew-prefix/INSTALL_RECEIPT.json '{"homebrew_version":"7.0.2","source":{"tap":"fengmk2/core"}}'`


## `vpt write-file brew-prefix/node_modules/vite-plus/package.json '{"name":"vite-plus"}'`


## `vpt write-file brew-prefix/node_modules/vite-plus/dist/bin.js 'console.log('\''bundled CLI'\'');'`


## `VP_HOME=${workspace}/home VP_NODE_MANAGER=no VP_PM_MANAGER=no ./brew-prefix/bin/vp --help`


## `VP_HOME=${workspace}/home ./home/bin/vp upgrade`

**Exit code:** 1

```
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.
```

## `VP_HOME=${workspace}/home ./home/bin/vp upgrade --force`

**Exit code:** 1

```
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.
```

## `VP_HOME=${workspace}/home ./home/bin/vp upgrade 0.0.1 --registry http://127.0.0.1:9`

**Exit code:** 1

```
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.
```

## `VP_HOME=${workspace}/home ./home/bin/vp upgrade --rollback`

**Exit code:** 1

```
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.
```

## `VP_HOME=${workspace}/home ./home/bin/vp upgrade --silent`

**Exit code:** 1

```
error: Upgrade error: Homebrew manages this installation. Run `brew upgrade vite-plus` to update it.
```

## `VP_HOME=${workspace}/home ./home/bin/vp upgrade --check`

```
info: Homebrew manages this installation. Run `brew outdated vite-plus` to check for updates.
```

## `VP_HOME=${workspace}/home ./home/bin/vp upgrade --check --silent`

```
```

## `VP_HOME=${workspace}/home ./home/bin/vp upgrade --background-check`

```
```

## `VP_HOME=${workspace}/home ./home/bin/vp env off`

```
VITE+ - The Unified Toolchain for the Web

✓ Node.js and package-manager management set to system-first.

Selected commands and shims will now prefer system tools, falling back to managed tools.

Run `vp env on` to always use Vite+ managed tools.
```

## `vpt stat-file home/current --assert missing`

```
home/current: missing
```

## `vpt stat-file home/cache/upgrade-check.json --assert missing`

```
home/cache/upgrade-check.json: missing
```

## `vpt stat-file brew-prefix/bin/.vp-setup-complete --assert missing`

```
brew-prefix/bin/.vp-setup-complete: missing
```

## `VP_HOME=${workspace}/home ./home/bin/vp implode --yes`

Cleanup removes user data and leaves the Homebrew package installed

```
✓ Vite+ removed 12 shims from <workspace>/home/bin
✓ Removed <workspace>/home

✓ Vite+ removed its managed files and shell entries from your system.
note: The Homebrew package remains installed. Run `brew uninstall vite-plus` to remove it.
note: To run `vp` again, restart your terminal or run `hash -r` in Bash. The remaining Homebrew package will start setup again.
note: Restart your terminal to apply shell changes.
```

## `vpt stat-file home --assert missing`

```
home: missing
```

## `vpt stat-file brew-prefix/bin/vp --assert file`

```
brew-prefix/bin/vp: file
```

## `vpt stat-file brew-prefix/INSTALL_RECEIPT.json --assert file`

```
brew-prefix/INSTALL_RECEIPT.json: file
```

## `vpt stat-file brew-prefix/node_modules/vite-plus/dist/bin.js --assert file`

```
brew-prefix/node_modules/vite-plus/dist/bin.js: file
```
