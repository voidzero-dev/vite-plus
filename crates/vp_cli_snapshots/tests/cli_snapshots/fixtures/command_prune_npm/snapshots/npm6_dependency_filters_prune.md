# npm6_dependency_filters_prune

## `vpt json-edit package.json packageManager npm@6.14.18`


## `vp install --ignore-scripts --silent`


## `vpt stat-file node_modules/test-vite-plus-package/package.json --assert file`

```
node_modules/test-vite-plus-package/package.json: file
```

## `vp pm prune --prod -- --silent`

```
removed 1 package in <duration>
```

## `vpt stat-file node_modules/test-vite-plus-package/package.json --assert missing`

```
node_modules/test-vite-plus-package/package.json: missing
```

## `vpt stat-file node_modules/testnpm2/package.json --assert file`

```
node_modules/testnpm2/package.json: file
```

## `vpt stat-file node_modules/test-vite-plus-package-optional/package.json --assert file`

```
node_modules/test-vite-plus-package-optional/package.json: file
```
