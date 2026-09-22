# yarn_2_drops_lockfile_only

## `vpt write-file .node-version '22.18.0
'`


## `vpt json-edit package.json packageManager yarn@2.4.2`


## `vp add ./dep --lockfile-only`

Yarn 2 warns and installs normally because add does not support --mode

```
warn: yarn <3 does not support --lockfile-only.
➤ YN0000: ┌ Resolution step
➤ YN0013: │ install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: Done in <duration>
```

## `vpt stat-file yarn.lock --assert file`

```
yarn.lock: file
```

## `node -p require('./node_modules/install-option-dep/package.json').version`

```
1.0.0
```

## `vpt rm -rf node_modules yarn.lock .yarn`


## `vp install ./dep-v2 --lockfile-only`

positional install uses the same warning-and-drop policy

```
VITE+ - The Unified Toolchain for the Web

warn: yarn <3 does not support --lockfile-only.
➤ YN0000: ┌ Resolution step
➤ YN0013: │ install-option-dep@file:./dep-v2#./dep-v2::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ install-option-dep@file:./dep-v2#./dep-v2::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: Done in <duration>
```

## `vpt stat-file yarn.lock --assert file`

```
yarn.lock: file
```

## `node -p require('./node_modules/install-option-dep/package.json').version`

```
2.0.0
```
