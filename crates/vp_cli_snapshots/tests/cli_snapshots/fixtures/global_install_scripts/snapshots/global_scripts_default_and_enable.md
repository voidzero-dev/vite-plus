# global_scripts_default_and_enable

Global installs and updates skip direct and transitive scripts by default and warn last. --run-scripts runs all scripts with the selected Node.js, including Node 20.

## `vpt mkdir -p tarballs`


## `npm pack ./native-addon --ignore-scripts --pack-destination tarballs`


## `npm pack ./my-cli --ignore-scripts --pack-destination tarballs`


## `vpt json-edit my-cli/package.json version 2.0.0`


## `npm pack ./my-cli --ignore-scripts --pack-destination tarballs`


## `vp install -g my-cli@1.0.0 native-addon@1.0.0`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 2 global packages with Node.js <version>
✓ Installed my-cli 1.0.0
  Bins: my-cli

✓ Installed native-addon 1.0.0
warning: Lifecycle scripts were skipped for: my-cli, native-addon.
To allow them, reinstall with:
  vp install -g my-cli@1.0.0 --run-scripts
warning: Lifecycle scripts were skipped for: native-addon.
To allow them, reinstall with:
  vp install -g native-addon@1.0.0 --run-scripts
```

## `my-cli skipped skipped`

```
my-cli: skipped; native-addon: skipped
```

## `npm_config_ignore_scripts=true npm_config_dangerously_allow_all_scripts=false vp install -g my-cli@1.0.0 --run-scripts`

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed my-cli 1.0.0
  Bins: my-cli
```

## `my-cli ran ran`

```
my-cli: ran; native-addon: ran
```

## `vp env install node@20.19.6`


## `vp add -g my-cli@1.0.0 --node 20.19.6 --ignore-scripts`

```
info: Installing 1 global package with Node.js <version>
✓ Installed my-cli 1.0.0
  Bins: my-cli
warning: Lifecycle scripts were skipped for: my-cli, native-addon.
To allow them, reinstall with:
  vp install -g my-cli@1.0.0 --node <version> --run-scripts
```

## `my-cli skipped skipped`

```
my-cli: skipped; native-addon: skipped
```

## `vp add -g my-cli@1.0.0 --node 20.19.6 --run-scripts`

```
info: Installing 1 global package with Node.js <version>
✓ Installed my-cli 1.0.0
  Bins: my-cli
```

## `my-cli ran ran`

```
my-cli: ran; native-addon: ran
```

## `vp update -g my-cli@2.0.0`

```
info: Updating 1 global package with Node.js <version>
✓ Updated my-cli to 2.0.0
  Bins: my-cli
warning: Lifecycle scripts were skipped for: my-cli, native-addon.
To allow them, reinstall with:
  vp install -g my-cli@2.0.0 --run-scripts
```

## `my-cli skipped skipped`

```
my-cli: skipped; native-addon: skipped
```

## `vp update -g my-cli@1.0.0 --run-scripts`

```
info: Updating 1 global package with Node.js <version>
✓ Updated my-cli to 1.0.0
  Bins: my-cli
```

## `my-cli ran ran`

```
my-cli: ran; native-addon: ran
```
