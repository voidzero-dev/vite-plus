# yarn_3_lockfile_only

## `vpt write-file .node-version '22.18.0
'`


## `vpt json-edit package.json packageManager yarn@3.0.0`


## `vp add ./dep --lockfile-only`

Yarn 3 supports update-lockfile without installing packages

```
➤ YN0000: ┌ Resolution step
➤ YN0013: │ install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0073: │ Skipped due to mode=update-lockfile
➤ YN0000: └ Completed
➤ YN0000: Done with warnings in <duration>
```

## `vpt stat-file yarn.lock --assert file`

```
yarn.lock: file
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vpt rm -rf yarn.lock .yarn`


## `vp install ./dep-v2 --lockfile-only`

positional install preserves lockfile-only at the first supported Yarn release

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: ┌ Resolution step
➤ YN0013: │ install-option-dep@file:./dep-v2#./dep-v2::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ install-option-dep@file:./dep-v2#./dep-v2::hash=<hash>&locator=install-package-options%40workspace%3A. can't be found in the cache and will be fetched from the disk
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0073: │ Skipped due to mode=update-lockfile
➤ YN0000: └ Completed
➤ YN0000: Done with warnings in <duration>
```

## `vpt stat-file yarn.lock --assert file`

```
yarn.lock: file
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```
