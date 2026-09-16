# yarn_berry_lockfile_only

## `vpt json-edit package.json packageManager yarn@4.10.3`


## `vp install ./dep --lockfile-only --ignore-scripts`

use update-lockfile rather than skip-build when both options are supplied

```
VITE+ - The Unified Toolchain for the Web

warn: yarn@2+ --mode can only be specified once; --lockfile-only takes priority over --ignore-scripts
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0073: │ Skipped due to mode=update-lockfile
➤ YN0000: └ Completed
➤ YN0000: · Done with warnings in <duration> <duration>
```

## `vpt stat-file yarn.lock --assert file`

```
yarn.lock: file
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vpt stat-file .pnp.cjs --assert missing`

```
.pnp.cjs: missing
```
