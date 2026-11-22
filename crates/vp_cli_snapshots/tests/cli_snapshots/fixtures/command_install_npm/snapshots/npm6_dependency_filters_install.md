# npm6_dependency_filters_install

## `vpt json-edit package.json packageManager npm@6.14.18`


## `vpt json-edit package.json dependencies.prod-only-fixture file:./prod-dep`


## `vpt json-edit package.json devDependencies.dev-only-fixture file:./dev-dep`


## `vpt json-edit package.json optionalDependencies.is-number 6.0.0`


## `vp install -P --ignore-scripts --silent`

npm 6 uses --production to omit dev dependencies

```
added 2 packages from 3 contributors in <duration>
```

## `vpt stat-file node_modules/prod-only-fixture/package.json --assert file`

```
node_modules/prod-only-fixture/package.json: file
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert missing`

```
node_modules/dev-only-fixture/package.json: missing
```

## `vpt stat-file node_modules/is-number/package.json --assert file`

```
node_modules/is-number/package.json: file
```

## `NODE_ENV=production npm_config_production=true vp install -D --frozen-lockfile --ignore-scripts --silent`

npm 6 ci includes dev dependencies despite production settings

```
added 3 packages in <duration>
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert file`

```
node_modules/dev-only-fixture/package.json: file
```

## `vpt rm -rf node_modules`


## `vp install --no-optional --ignore-scripts --silent`

```
added 2 packages in <duration>
```

## `vpt stat-file node_modules/is-number/package.json --assert missing`

```
node_modules/is-number/package.json: missing
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert file`

```
node_modules/dev-only-fixture/package.json: file
```

## `vp install -P --no-optional --frozen-lockfile --ignore-scripts --silent`

```
added 1 packages in <duration>
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert missing`

```
node_modules/dev-only-fixture/package.json: missing
```

## `vpt stat-file node_modules/is-number/package.json --assert missing`

```
node_modules/is-number/package.json: missing
```

## `NODE_ENV=production vp install -P -D --ignore-scripts --silent`

explicit inclusion wins when both dependency flags are supplied

```
added 2 packages from 3 contributors in <duration>
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert file`

```
node_modules/dev-only-fixture/package.json: file
```

## `NODE_ENV=production vp install -P -D --no-optional --frozen-lockfile --ignore-scripts --silent`

```
added 2 packages in <duration>
```

## `vpt stat-file node_modules/prod-only-fixture/package.json --assert file`

```
node_modules/prod-only-fixture/package.json: file
```

## `vpt stat-file node_modules/dev-only-fixture/package.json --assert file`

```
node_modules/dev-only-fixture/package.json: file
```

## `vpt stat-file node_modules/is-number/package.json --assert missing`

```
node_modules/is-number/package.json: missing
```
