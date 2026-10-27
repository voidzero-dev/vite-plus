# npm6_dependency_filters_update

## `vpt json-edit package.json packageManager npm@6.14.18`


## `vpt json-edit package.json devDependencies '{"yocto-queue":"1.0.0"}'`


## `vp install --ignore-scripts --silent`


## `vpt grep-file node_modules/yocto-queue/package.json '"version": "1.0.0"'`

```
node_modules/yocto-queue/package.json: found "\"version\": \"1.0.0\""
```

## `vpt json-edit package.json devDependencies.yocto-queue ^1.0.0`


## `NODE_ENV=production vp update -D -- --ignore-scripts --silent`

npm 6 uses --also=dev to include development updates

```
 yocto-queue@1.2.2
updated 1 package in <duration>
```

## `vpt grep-file node_modules/yocto-queue/package.json '"version": "1.2.2"'`

```
node_modules/yocto-queue/package.json: found "\"version\": \"1.2.2\""
```

## `vp update -P`

reject --prod because npm 6 selects updates through outdated, whose --production still reports installed devDependencies

**Exit code:** 1

```
npm < 7 does not support --prod.
```
