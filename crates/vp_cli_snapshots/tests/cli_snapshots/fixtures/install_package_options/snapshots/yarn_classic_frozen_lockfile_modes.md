# yarn_classic_frozen_lockfile_modes

## `vpt json-edit package.json packageManager yarn@1.22.22`


## `vp install ./dep`


## `vpt cp yarn.lock before.lock`


## `vp install ./dep-v2 --frozen-lockfile`

Classic add preserves the lockfile but allows manifest changes

```
VITE+ - The Unified Toolchain for the Web

yarn add <version>
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Building fresh packages...

success Saved 1 new dependency.
info Direct dependencies
└─ install-option-dep@2.0.0
info All dependencies
└─ install-option-dep@2.0.0

Done in <duration>.
```

## `node -e 'const fs = require('\''node:fs'\''); if ('\!'fs.readFileSync('\''yarn.lock'\'').equals(fs.readFileSync('\''before.lock'\''))) process.exit(1); console.log('\''lockfile unchanged'\'');'`

```
lockfile unchanged
```

## `node -p require('./package.json').dependencies['install-option-dep']`

```
./dep-v2
```

## `vp install --frozen-lockfile`

package-free installs still enforce the frozen lockfile

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

yarn install <version>
[1/4] Resolving packages...
error Your lockfile needs to be updated, but yarn was run with `--frozen-lockfile`.
info Visit https://yarnpkg.com/en/docs/cli/install for documentation about this command.
```

## `node -e 'const fs = require('\''node:fs'\''); if ('\!'fs.readFileSync('\''yarn.lock'\'').equals(fs.readFileSync('\''before.lock'\''))) process.exit(1); console.log('\''lockfile unchanged'\'');'`

```
lockfile unchanged
```

## `vp install --no-frozen-lockfile`

package-free installs can explicitly allow lockfile changes

```
VITE+ - The Unified Toolchain for the Web

yarn install <version>
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Building fresh packages...

success Saved lockfile.

Done in <duration>.
```

## `node -p require('./node_modules/install-option-dep/package.json').version`

```
2.0.0
```

## `vp add ./dep --no-frozen-lockfile`

Classic add warns and drops the unsupported negated flag

```
warn: yarn does not support --no-frozen-lockfile.
yarn add <version>
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Building fresh packages...

success Saved lockfile.
success Saved 1 new dependency.
info Direct dependencies
└─ install-option-dep@1.0.0
info All dependencies
└─ install-option-dep@1.0.0

Done in <duration>.
```

## `node -p require('./node_modules/install-option-dep/package.json').version`

```
1.0.0
```
