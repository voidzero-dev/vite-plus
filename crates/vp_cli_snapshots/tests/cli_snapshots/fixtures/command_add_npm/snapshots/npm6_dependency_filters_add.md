# npm6_dependency_filters_add

## `vpt json-edit package.json packageManager npm@6.14.18`


## `vpt json-edit package.json dependencies.is-number 6.0.0`


## `vp install --ignore-scripts --silent`


## `vp pm pack -- ./optional-parent --ignore-scripts`


## `vp add ./optional-parent-1.0.0.tgz --ignore-scripts --silent`

without a filter, the optional dependency is installed inside the added package

```
 optional-parent@1.0.0
added 2 packages from 3 contributors in <duration>
```

## `vpt stat-file node_modules/optional-parent/node_modules/is-number/package.json --assert file`

```
node_modules/optional-parent/node_modules/is-number/package.json: file
```

## `vpt rm -rf node_modules/optional-parent package-lock.json`


## `vp add ./optional-parent-1.0.0.tgz --no-optional --ignore-scripts --silent`

```
 optional-parent@1.0.0
added 1 package in <duration>
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
 optional-parent@1.0.0
added 1 package in <duration>
```

## `vpt stat-file node_modules/optional-parent/package.json --assert file`

```
node_modules/optional-parent/package.json: file
```

## `vpt stat-file node_modules/optional-parent/node_modules/is-number/package.json --assert missing`

```
node_modules/optional-parent/node_modules/is-number/package.json: missing
```

## `vpt stat-file node_modules/is-number/package.json --assert file`

```
node_modules/is-number/package.json: file
```
