# yarn_berry_frozen_lockfile_modes

## `vpt json-edit package.json packageManager yarn@4.10.3`


## `vpt write-file .yarnrc.yml 'nodeLinker: node-modules
'`


## `vp install ./dep --frozen-lockfile`

named-package installs reject add's unsupported immutable option

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

yarn >= 2 does not support --frozen-lockfile.
```

## `vpt stat-file yarn.lock --assert missing`

```
yarn.lock: missing
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vp install ./dep`

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration>
```

## `vpt json-edit package.json dependencies.install-option-dep file:./dep-v2`


## `vp install --frozen-lockfile`

package-free installs map frozen-lockfile to immutable

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep-v2#./dep-v2::hash=<hash>&locator=install-package-options%40workspace%3A.
➤ YN0085: │ - install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed

➤ YN0000: ┌ Post-resolution validation
➤ YN0000: │ @@ -4,18 +4,17 @@
➤ YN0000: │  __metadata:
➤ YN0000: │    version: 8
➤ YN0000: │    cacheKey: 10c0
➤ YN0000: │
➤ YN0028: │ -"install-option-dep@file:./dep::locator=install-package-options%40workspace%3A.":
➤ YN0028: │ -  version: 1.0.0
➤ YN0028: │ -  resolution: "install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A."
➤ YN0028: │ -  checksum: <hash>
➤ YN0028: │ +"install-option-dep@file:./dep-v2::locator=install-package-options%40workspace%3A.":
➤ YN0028: │ +  version: 2.0.0
➤ YN0028: │ +  resolution: "install-option-dep@file:./dep-v2#./dep-v2::hash=<hash>&locator=install-package-options%40workspace%3A."
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
➤ YN0000: · Failed with errors in <duration>
```

## `vpt grep-file yarn.lock install-option-dep@file:./dep::`

the immutable failure leaves the original lockfile entry in place

```
yarn.lock: found "install-option-dep@file:./dep::"
```

## `vp install --no-frozen-lockfile`

package-free installs map no-frozen-lockfile to no-immutable

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0085: │ + install-option-dep@file:./dep-v2#./dep-v2::hash=<hash>&locator=install-package-options%40workspace%3A.
➤ YN0085: │ - install-option-dep@file:./dep#./dep::hash=<hash>&locator=install-package-options%40workspace%3A.
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0013: │ A package was added to the project, and one was removed (+ <size> KiB).
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration>
```

## `node -p require('./node_modules/install-option-dep/package.json').version`

```
2.0.0
```

## `vp add ./dep --no-frozen-lockfile`

direct add also rejects the unsupported immutable override

**Exit code:** 1

```
yarn does not support --no-frozen-lockfile.
```

## `node -p require('./node_modules/install-option-dep/package.json').version`

```
2.0.0
```
