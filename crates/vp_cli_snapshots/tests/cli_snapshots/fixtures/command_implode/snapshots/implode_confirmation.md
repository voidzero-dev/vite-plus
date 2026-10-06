# implode_confirmation

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
