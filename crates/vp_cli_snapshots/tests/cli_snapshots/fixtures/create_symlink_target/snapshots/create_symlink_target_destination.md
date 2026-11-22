# create_symlink_target_destination

The overwrite prompt shows the link destination even with a trailing separator; cancellation preserves the link and its contents.

## `node setup.mjs`


## `vp create vite:application --directory new-project/ --interactive --no-agent --no-editor --no-git --no-hooks`

**→ expect-milestone:** `select:select:0`

```
VITE+ - The Unified Toolchain for the Web

› Target path "<workspace>/new-project" is a symbolic link to "<workspace>/linked-directory". Please choose how to proceed:
  › Cancel operation
    Remove symbolic link and continue
  ↑/↓ navigate · Enter confirm
```

**← write-key:** `enter`

```
VITE+ - The Unified Toolchain for the Web

◇ Target path "<workspace>/new-project" is a symbolic link to "<workspace>/linked-directory". Please choose how to proceed:
  Cancel operation

Operation cancelled
```

## `vpt stat-file new-project --assert symlink`

```
new-project: symlink
```

## `vpt print-file linked-directory/keep.txt`

```
keep
```
