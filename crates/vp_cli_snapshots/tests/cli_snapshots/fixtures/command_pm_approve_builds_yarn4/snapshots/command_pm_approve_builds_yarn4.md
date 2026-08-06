# command_pm_approve_builds_yarn4

## `vp pm approve-builds`

yarn Berry warn — points at dependenciesMeta["<pkg>"].built

```
warn: yarn does not run third-party build scripts by default. To allow a package, set `dependenciesMeta["<package>"].built: true` in package.json.
```

## `vp pm approve-builds esbuild`

same warn (no native command on yarn)

```
warn: yarn does not run third-party build scripts by default. To allow a package, set `dependenciesMeta["<package>"].built: true` in package.json.
```

## `vp pm approve-builds esbuild -- --silent`

extras trigger the dropped-pass-through warn

```
warn: yarn does not run third-party build scripts by default. To allow a package, set `dependenciesMeta["<package>"].built: true` in package.json.
warn: Ignoring pass-through args (--silent): this package manager has no native approve-builds command to forward them to.
```
