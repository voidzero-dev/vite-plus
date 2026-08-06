# command_exec_cwd

## `node setup.js`


## `vp exec -c 'basename $(pwd)'`

cwd is package root

```
workspace
```

## `cd src && vp exec -c 'basename $(pwd)'`

cwd preserved in subdirectory

```
src
```

## `cd src/nested && vp exec -c 'basename $(pwd)'`

cwd preserved in nested subdirectory

```
nested
```

## `cd src && vp exec node -e 'const p = require('\''path'\''); console.log(p.basename(process.cwd()))'`

non-shell mode also preserves cwd

```
src
```
