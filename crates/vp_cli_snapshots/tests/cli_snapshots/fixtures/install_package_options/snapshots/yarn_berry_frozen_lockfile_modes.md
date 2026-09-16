# yarn_berry_frozen_lockfile_modes

## `vpt json-edit package.json packageManager yarn@4.10.3`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
'`


## `vp install ./dep --frozen-lockfile`

named-package installs use add, which has no immutable option

```
VITE+ - The Unified Toolchain for the Web

warn: yarn does not support --frozen-lockfile.
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep#./dep::hash=8572a9&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt cp yarn.lock before.lock`


## `vpt json-edit package.json dependencies.install-option-dep file:./dep-v2`


## `vp install --frozen-lockfile`

package-free installs map frozen-lockfile to immutable

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep-v2#./dep-v2::hash=ca2da2&locator=install-package-options%40workspace%3A.
➤ YN0085: │ - install-option-dep@file:./dep#./dep::hash=8572a9&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed

➤ YN0000: ┌ Post-resolution validation
➤ YN0000: │ @@ -4,18 +4,17 @@
➤ YN0000: │  __metadata:
➤ YN0000: │    version: 8
➤ YN0000: │    cacheKey: 10c0
➤ YN0000: │
➤ YN0028: │ -"install-option-dep@file:./dep::locator=install-package-options%40workspace%3A.":
➤ YN0028: │ -  version: 1.0.0
➤ YN0028: │ -  resolution: "install-option-dep@file:./dep#./dep::hash=8572a9&locator=install-package-options%40workspace%3A."
➤ YN0028: │ -  checksum: 10c0/32675cb2e55f886e9f975fdaefa9cb706071880078bda04af25dd0343941bf33780453c0ec848b30d3aa53cb9d5d5b3a6aeef902552100f255ed35fac8a8ff06
➤ YN0028: │ +"install-option-dep@file:./dep-v2::locator=install-package-options%40workspace%3A.":
➤ YN0028: │ +  version: 2.0.0
➤ YN0028: │ +  resolution: "install-option-dep@file:./dep-v2#./dep-v2::hash=ca2da2&locator=install-package-options%40workspace%3A."
➤ YN0000: │    languageName: node
➤ YN0000: │    linkType: hard
➤ YN0000: │
➤ YN0000: │  "install-package-options@workspace:.":
➤ YN0000: │    version: 0.0.0-use.local
➤ YN0000: │    resolution: "install-package-options@workspace:."
➤ YN0000: │    dependencies:
➤ YN0028: │ -    install-option-dep: ./dep
➤ YN0028: │ +    install-option-dep: "file:./dep-v2"
➤ YN0000: │    languageName: unknown
➤ YN0000: │    linkType: soft
➤ YN0000: │
➤ YN0028: │ The lockfile would have been modified by this install, which is explicitly forbidden.
➤ YN0000: └ Completed
➤ YN0000: · Failed with errors in <duration> <duration>
```

## `node -e 'const fs = require('\''node:fs'\''); if ('\!'fs.readFileSync('\''yarn.lock'\'').equals(fs.readFileSync('\''before.lock'\''))) process.exit(1); console.log('\''lockfile unchanged'\'');'`

```
lockfile unchanged
```

## `vp install --no-frozen-lockfile`

package-free installs map no-frozen-lockfile to no-immutable

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep-v2#./dep-v2::hash=ca2da2&locator=install-package-options%40workspace%3A.
➤ YN0085: │ - install-option-dep@file:./dep#./dep::hash=8572a9&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `node -p require('./node_modules/install-option-dep/package.json').version`

```
2.0.0
```

## `vp add ./dep --no-frozen-lockfile`

direct add also warns for the unsupported immutable override

```
warn: yarn does not support --no-frozen-lockfile.
➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep#./dep::hash=8572a9&locator=install-package-options%40workspace%3A.
➤ YN0085: │ - install-option-dep@file:./dep-v2#./dep-v2::hash=ca2da2&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```
