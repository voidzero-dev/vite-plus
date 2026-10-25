# implode_confirmation

## `vpt rm $VP_HOME/cache/.vp-owned`


## `vp implode --yes`

A nested cache needs its own ownership marker before any deletion

**Exit code:** 1

```
error: Refusing to remove <home>/.vite-plus/cache: Vite+ directory ownership could not be verified. Unmarked directories, including older installations, must be inspected and removed manually.
```

## `vpt write-file $VP_HOME/cache/.vp-owned 'vite-plus-owned-directory
'`


## `vp implode`

The old confirmation word leaves the installation intact

**→ expect-milestone:** `text:implode:ready`

```
warn: This will completely remove vite-plus from your system!

  Directories to remove:
    - <home>/.vite-plus
  Shim files to remove from: <home>/.vite-plus/bin

Type boom to confirm:
```

**← write-line:** `uninstall`

```
warn: This will completely remove vite-plus from your system!

  Directories to remove:
    - <home>/.vite-plus
  Shim files to remove from: <home>/.vite-plus/bin

Type boom to confirm:
uninstall
info: Aborted.
```

## `vpt stat-file $VP_HOME --assert dir`

```
<home>/.vite-plus: dir
```

## `vp implode`

The command name does not confirm removal

**→ expect-milestone:** `text:implode:ready`

```
warn: This will completely remove vite-plus from your system!

  Directories to remove:
    - <home>/.vite-plus
  Shim files to remove from: <home>/.vite-plus/bin

Type boom to confirm:
```

**← write-line:** `implode`

```
warn: This will completely remove vite-plus from your system!

  Directories to remove:
    - <home>/.vite-plus
  Shim files to remove from: <home>/.vite-plus/bin

Type boom to confirm:
implode
info: Aborted.
```

## `vpt stat-file $VP_HOME --assert dir`

```
<home>/.vite-plus: dir
```

## `vp implode`

Empty input cancels removal

**→ expect-milestone:** `text:implode:ready`

```
warn: This will completely remove vite-plus from your system!

  Directories to remove:
    - <home>/.vite-plus
  Shim files to remove from: <home>/.vite-plus/bin

Type boom to confirm:
```

**← write-key:** `enter`

```
warn: This will completely remove vite-plus from your system!

  Directories to remove:
    - <home>/.vite-plus
  Shim files to remove from: <home>/.vite-plus/bin

Type boom to confirm:

info: Aborted.
```

## `vpt stat-file $VP_HOME --assert dir`

```
<home>/.vite-plus: dir
```

## `vp implode`

boom confirms removal


## `vpt stat-file $VP_HOME --assert missing`

```
<home>/.vite-plus: missing
```
