# npm10_dependency_filters_add

## `vpt json-edit package.json dependencies.is-number 6.0.0`


## `vp install --ignore-scripts --silent`


## `vp pm pack -- ./optional-parent --ignore-scripts`


## `vp add ./optional-parent-1.0.0.tgz --ignore-scripts --silent`

```
```

## `vpt stat-file node_modules/optional-parent/node_modules/is-number/package.json --assert file`

```
node_modules/optional-parent/node_modules/is-number/package.json: file
```

## `vpt rm -rf node_modules/optional-parent package-lock.json`


## `vp add ./optional-parent-1.0.0.tgz --no-optional --ignore-scripts --silent`

```
```

## `vpt stat-file node_modules/optional-parent/package.json --assert file`

```
node_modules/optional-parent/package.json: file
```

## `vpt stat-file node_modules/optional-parent/node_modules/is-number/package.json --assert missing`

```
node_modules/optional-parent/node_modules/is-number/package.json: missing
```

## `vpt rm -rf node_modules/optional-parent package-lock.json`


## `vp install ./optional-parent-1.0.0.tgz --no-optional --ignore-scripts --silent`

```
```

## `vpt stat-file node_modules/optional-parent/package.json --assert file`

```
node_modules/optional-parent/package.json: file
```

## `vpt stat-file node_modules/optional-parent/node_modules/is-number/package.json --assert missing`

```
node_modules/optional-parent/node_modules/is-number/package.json: missing
```
