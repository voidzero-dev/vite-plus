# npm_lockfile_uses_bundled_version

## `NPM_CONFIG_REGISTRY=http://127.0.0.1:9 node assert-npm.cjs`

A Node-only devEngines declaration and npm lockfile use bundled npm without querying the registry

```
npm/npx and vp use Node bundled npm
```

## `vp pm patch example`

Bundled npm retains its version gates for unsupported commands

```
warn: npm does not have a 'patch' command.
```

## `vp pm approve-builds`

Bundled npm does not invoke approval commands added in later npm releases

```
warn: npm runs lifecycle scripts by default. Upgrade to npm >= 11.16.0 for `npm approve-scripts`/`deny-scripts`, or set `ignore-scripts=true` in .npmrc and rebuild approved packages with `vp pm rebuild <package>`.
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
