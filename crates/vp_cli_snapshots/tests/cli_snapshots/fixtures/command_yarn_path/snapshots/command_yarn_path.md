# command_yarn_path

## `vp env exec --package-manager yarn@3.8.7 node setup.cjs 3.8.7`


## `vp env exec --package-manager yarn@2.4.2 node setup.cjs 2.4.2`


## `vp env exec --package-manager yarn@4.18.0 node setup.cjs 4.18.0`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
yarnPath: .yarn/releases/yarn-3.8.7.cjs
'`


## `vp install`


## `vpt mkdir -p ../linked-lib`


## `vpt write-file ../linked-lib/package.json '{"name":"linked-lib","version":"1.0.0"}
'`


## `vp link ../linked-lib`


## `vp unlink linked-lib`

The Yarn 2 launcher delegates unlink to Yarn 3.

```
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: Done in <duration>
```

## `node verify.cjs unlink`

```
The delegated Yarn removed the portal resolution.
```

## `node verify.cjs patch`

The same launcher must let Yarn 3 save a patch.

```
The delegated Yarn saved and registered the patch.
```

## `vpt write-file .yarnrc.yml 'yarnPath: .yarn/releases/yarn-3.8.7.cjs
ignorePath: true
'`


## `vp unlink linked-lib`

ignorePath keeps Yarn 2 active, so unlink is skipped.

```
warn: yarn unlink requires Yarn Classic or yarn >= 3.0.0. Skipping.
```

## `vp pm patch-commit unused`

ignorePath also keeps the patch saving gate on Yarn 2.

```
warn: yarn patch-commit --save requires yarn >= 3.0.0. Skipping.
```

## `vpt write-file .yarnrc.yml 'yarnPath: .yarn/releases/yarn-3.8.7.cjs
'`


## `YARN_IGNORE_PATH=1 vp unlink linked-lib`

The environment override also disables delegation.

```
warn: yarn unlink requires Yarn Classic or yarn >= 3.0.0. Skipping.
```

## `vpt json-edit package.json packageManager yarn@3.8.7`


## `vpt write-file .yarnrc.yml 'yarnPath: .yarn/releases/yarn-2.4.2.cjs
'`


## `vp unlink linked-lib`

A newer launcher delegating to Yarn 2 must still skip unlink.

```
warn: yarn unlink requires Yarn Classic or yarn >= 3.0.0. Skipping.
```

## `vpt json-edit package.json packageManager yarn@4.18.0`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
yarnPath: .yarn/releases/yarn-3.8.7.cjs
'`


## `vp unlink linked-lib`

A Yarn 4 launcher must not inject Yarn 4 settings into the effective Yarn 3.

```
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ is-number@patch:is-number@npm%3A7.0.0#./.yarn/patches/is-number-npm-7.0.0-060086935c.patch::version=7.0.0&hash=6ab187&locator=command-yarn-path%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: Done in <duration>
```

## `vpt json-edit package.json packageManager yarn@2.4.2`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
yarnPath: .yarn/releases/yarn-4.18.0.cjs
'`


## `vp unlink linked-lib`

The Yarn 2 launcher must also accept the settings for its effective Yarn 4.

```
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ 2 packages were added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration>
```

## `vpt json-edit package.json packageManager yarn@3.8.7`


## `vpt write-file .yarnrc.yml 'yarnPath: .yarn/releases/missing.cjs
'`


## `vp unlink linked-lib`

A failed version query must report an error rather than fall back to the launcher version.

**Exit code:** 1

```
Failed to read Yarn version:
Internal Error: The "yarn-path" option has been set (in <workspace>/.yarnrc.yml), but the specified location doesn't exist (<workspace>/.yarn/releases/missing.cjs).
    at i (<home>/.vite-plus/package_manager/yarn/<version>/yarn/bin/yarn.js:447:11180)
    at async t (<home>/.vite-plus/package_manager/yarn/<version>/yarn/bin/yarn.js:447:10283)
```

## `vpt write-file .yarn/releases/invalid.cjs 'console.log('\''not-a-version'\'');
'`


## `vpt write-file .yarnrc.yml 'yarnPath: .yarn/releases/invalid.cjs
'`


## `vp unlink linked-lib`

An invalid version response must not pass the gate.

**Exit code:** 1

```
error: Invalid yarn version not-a-version: unexpected character 'n' while parsing major version number
```
