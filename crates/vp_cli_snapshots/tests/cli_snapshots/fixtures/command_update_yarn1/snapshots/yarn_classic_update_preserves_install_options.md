# yarn_classic_update_preserves_install_options

## `vpt json-edit package.json dependencies.is-number 6.0.0`


## `vpt json-edit package.json devDependencies.yocto-queue 0.1.0`


## `vpt json-edit package.json optionalDependencies.isarray 1.0.0`


## `vp install --ignore-scripts`

```
VITE+ - The Unified Toolchain for the Web

yarn install <version>
info No lockfile found.
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Building fresh packages...
warning Ignored scripts due to flag.
success Saved lockfile.

Done in <duration>.
```

## `vp update is-number@7.0.0 --prod -- --ignore-scripts`

Classic production mode excludes dev dependencies from installation during a named update

```
yarn upgrade <version>
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Rebuilding all packages...
warning Ignored scripts due to flag.
success Saved lockfile.
success Saved 1 new dependency.
info Direct dependencies
└─ is-number@7.0.0
info All dependencies
└─ is-number@7.0.0

Done in <duration>.
```

## `node -p require('./node_modules/is-number/package.json').version`

```
7.0.0
```

## `vpt stat-file node_modules/yocto-queue --assert missing`

```
node_modules/yocto-queue: missing
```

## `vpt stat-file node_modules/isarray/package.json --assert file`

```
node_modules/isarray/package.json: file
```

## `vp install --ignore-scripts`

```
VITE+ - The Unified Toolchain for the Web

yarn install <version>
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Building fresh packages...
warning Ignored scripts due to flag.

Done in <duration>.
```

## `vp update is-number@6.0.0 --no-optional -- --ignore-scripts`

optional dependencies are excluded from installation without removing their manifest entries

```
yarn upgrade <version>
[1/4] Resolving packages...
[2/4] Fetching packages...
[3/4] Linking dependencies...
[4/4] Rebuilding all packages...
warning Ignored scripts due to flag.
success Saved lockfile.
success Saved 1 new dependency.
info Direct dependencies
└─ is-number@6.0.0
info All dependencies
└─ is-number@6.0.0

Done in <duration>.
```

## `node -p require('./node_modules/is-number/package.json').version`

```
6.0.0
```

## `vpt stat-file node_modules/yocto-queue/package.json --assert file`

```
node_modules/yocto-queue/package.json: file
```

## `vpt stat-file node_modules/isarray --assert missing`

```
node_modules/isarray: missing
```

## `vpt print-file package.json`

```
{
  "dependencies": {
    "is-number": "6.0.0"
  },
  "devDependencies": {
    "yocto-queue": "0.1.0"
  },
  "name": "command-update-yarn1",
  "optionalDependencies": {
    "isarray": "1.0.0"
  },
  "packageManager": "yarn@1.22.22",
  "private": true,
  "version": "1.0.0"
}
```
