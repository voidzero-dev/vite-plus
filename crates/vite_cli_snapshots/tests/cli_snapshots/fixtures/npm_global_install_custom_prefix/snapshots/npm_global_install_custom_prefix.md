# npm_global_install_custom_prefix

## `vpt mkdir -p custom-prefix`

```
```

## `NPM_CONFIG_PREFIX=./custom-prefix npm install -g ./npm-global-custom-prefix-pkg`

Should install to custom prefix and create link

```

added 1 package in <duration>
Linked 'npm-global-custom-prefix-cli' to <home>/.vite-plus/bin/npm-global-custom-prefix-cli
```

## `vpt stat-file custom-prefix/bin/npm-global-custom-prefix-cli --assert file`

Verify installed to custom prefix

**Exit code:** 1

```
custom-prefix/bin/npm-global-custom-prefix-cli: symlink
stat-file assertion failed
```

## `vpt stat-file $VP_HOME/bin/npm-global-custom-prefix-cli --assert symlink`

Link should exist

```
<home>/.vite-plus/bin/npm-global-custom-prefix-cli: symlink
```

## `npm-global-custom-prefix-cli`

Should be callable via the link

```
npm-global-custom-prefix-cli works
```

## `vpt rm -f $VP_HOME/bin/npm-global-custom-prefix-cli`

Cleanup link

```
```

## `NPM_CONFIG_PREFIX=./custom-prefix npm uninstall -g npm-global-custom-prefix-pkg`

Cleanup

```

removed 1 package in <duration>
```

## `vpt rm -rf custom-prefix`

```
```
