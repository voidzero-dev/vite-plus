# implode_unowned_directory

## `vpt mkdir external`


## `vpt cp $VP_HOME/bin/vp external/vp`


## `vpt chmod +x external/vp`


## `vpt write-file $HOME/Documents/personal.txt 'keep my documents
'`


## `vpt write-file $HOME/.bashrc '# personal config
# Vite+ bin
. "$HOME/Documents/env"
'`


## `VP_HOME=${HOME}/Documents vp implode --yes`

An unrelated override fails before touching files or profiles

**Exit code:** 1

```
error: Refusing to remove <home>/Documents: Vite+ could not verify that it manages this directory. Check your directory overrides and inspect the directory before removing it manually.
```

## `vpt print-file $HOME/.bashrc`

```
# personal config
# Vite+ bin
. "$HOME/Documents/env"
```

## `VP_HOME=${HOME}/Documents ./external/vp`

Setup succeeds without claiming a nonempty personal directory


## `vpt stat-file $HOME/Documents/current/bin/.vp-setup-complete --assert file`

```
<home>/Documents/current/bin/.vp-setup-complete: file
```

## `VP_HOME=${HOME}/Documents vp implode --yes`

Only the newly owned cache is removed; the unmarked parent is preserved

```
✓ Cleaned ~/.bashrc
✓ Removed <home>/Documents/cache

✓ Vite+ removed its verified files and shell entries from your system.
note: Some paths were preserved to protect unverified data. Inspect them before removing anything:
  <home>/Documents/0.0.0-test
  <home>/Documents/bin
  <home>/Documents/config.json
  <home>/Documents/current
  <home>/Documents/env
  <home>/Documents/env.fish
  <home>/Documents/env.nu
  <home>/Documents/env.ps1
  <home>/Documents/fallback-bin
  <home>/Documents/personal.txt
  <home>/Documents/self-setup
  <home>/Documents/vp.ps1
note: Restart your terminal to apply shell changes.
```

## `vpt stat-file $HOME/Documents/cache --assert missing`

```
<home>/Documents/cache: missing
```

## `vpt stat-file $HOME/Documents/.vp-owned --assert missing`

```
<home>/Documents/.vp-owned: missing
```

## `vpt print-file $HOME/Documents/personal.txt`

```
keep my documents
```

## `vpt print-file $HOME/.bashrc`

```
# personal config
```
