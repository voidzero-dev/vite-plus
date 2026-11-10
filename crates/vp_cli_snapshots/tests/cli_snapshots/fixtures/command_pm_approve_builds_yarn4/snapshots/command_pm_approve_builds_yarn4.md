# command_pm_approve_builds_yarn4

## `vp pm approve-builds`

yarn Berry warn — points at dependenciesMeta["<pkg>"].built

```
warn: yarn has no native approve-builds command. To restrict third-party build scripts, set `enableScripts: false` in .yarnrc.yml, then allow individual packages with `dependenciesMeta["<package>"].built: true` in the root package.json.
```

## `vp pm approve-builds esbuild`

same warn (no native command on yarn)

```
warn: yarn has no native approve-builds command. To restrict third-party build scripts, set `enableScripts: false` in .yarnrc.yml, then allow individual packages with `dependenciesMeta["<package>"].built: true` in the root package.json.
```

## `vp pm approve-builds esbuild -- --silent`

extras trigger the dropped-pass-through warn

```
warn: yarn has no native approve-builds command. To restrict third-party build scripts, set `enableScripts: false` in .yarnrc.yml, then allow individual packages with `dependenciesMeta["<package>"].built: true` in the root package.json.
warn: Ignoring pass-through args (--silent): this package manager has no native approve-builds command to forward them to.
```
