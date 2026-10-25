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


## `VP_HOME=${HOME}/Documents ./external/vp`

A fresh install cannot claim a nonempty personal directory

**Exit code:** 1

```
error: Refusing to initialize <home>/Documents: the directory is not empty and Vite+ ownership could not be verified. Choose an empty installation directory.
```

## `VP_HOME=${HOME}/Documents vp implode --yes`

Changing VP_HOME cannot authorize recursive deletion

**Exit code:** 1

```
error: Refusing to remove <home>/Documents: Vite+ directory ownership could not be verified. Unmarked directories, including older installations, must be inspected and removed manually.
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
# Vite+ bin
. "$HOME/Documents/env"
```
