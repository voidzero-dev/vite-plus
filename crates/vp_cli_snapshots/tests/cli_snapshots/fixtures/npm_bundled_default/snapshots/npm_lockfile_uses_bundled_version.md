# npm_lockfile_uses_bundled_version

## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 node assert-npm.cjs`

A Node-only devEngines declaration and npm lockfile use bundled npm without querying the registry

```
npm/npx and vp use Node bundled npm
```

## `vp env default npm@10.5.0`


## `node assert-npm.cjs 10.5.0`

An npm default overrides the bundled version even when a lockfile exists

```
npm/npx and vp use configured npm <version>
```

## `vpt json-edit package.json devEngines.packageManager '{"name":"npm","version":"10.9.4"}'`


## `node assert-npm.cjs 10.9.4`

A devEngines npm version overrides the default

```
npm/npx and vp use configured npm <version>
```

## `vpt json-edit package.json packageManager npm@10.8.2`


## `node assert-npm.cjs 10.8.2`

A top-level npm pin still has priority

```
npm/npx and vp use configured npm <version>
```
