# npm_global_uninstall_prefix_shorthand

## `npm install -g --prefix ./custom-prefix ./npm-global-prefix-pkg`

```

added 1 package in <duration>
Linked 'npm-global-prefix-cli' to <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert symlink`

```
<home>/.vite-plus/bin/npm-global-prefix-cli: symlink
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert file`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: file
```

## `NPM_CONFIG_PREFIX=${workspace}/other-prefix npm uninstall -g -C ./custom-prefix npm-global-prefix-pkg --dry-run`

A shorthand prefix override must still preserve links during dry-run

```
remove npm-global-prefix-pkg 1.0.0

removed 1 package in <duration>
```

## `npm-global-prefix-cli`

```
npm-global-prefix-cli works
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert file`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: file
```

## `NPM_CONFIG_PREFIX=${workspace}/other-prefix npm uninstall -g -C ./custom-prefix npm-global-prefix-pkg`

-C must override the environment prefix for uninstall link cleanup

```

removed 1 package in <duration>
Removed link 'npm-global-prefix-cli' from <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file custom-prefix/lib/node_modules/npm-global-prefix-pkg --assert missing`

```
custom-prefix/lib/node_modules/npm-global-prefix-pkg: missing
```

## `vpt stat-file custom-prefix/bin/npm-global-prefix-cli --assert missing`

```
custom-prefix/bin/npm-global-prefix-cli: missing
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert missing`

```
<home>/.vite-plus/bin/npm-global-prefix-cli: missing
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert missing`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: missing
```

## `vpt write-file prefix.npmrc 'prefix=${HOME}/npm-prefix
'`

```
```

## `npm_config_userconfig=${workspace}/prefix.npmrc npm install -g -C=./custom-prefix ./npm-global-prefix-pkg`

-C= must override the user npmrc prefix when creating links

```

added 1 package in <duration>
Linked 'npm-global-prefix-cli' to <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert symlink`

```
<home>/.vite-plus/bin/npm-global-prefix-cli: symlink
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert file`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: file
```

## `npm-global-prefix-cli`

```
npm-global-prefix-cli works
```

## `npm_config_userconfig=${workspace}/prefix.npmrc npm -C ./custom-prefix uninstall -g npm-global-prefix-pkg`

-C before the subcommand must also override the user npmrc prefix for cleanup

```

removed 1 package in <duration>
Removed link 'npm-global-prefix-cli' from <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file custom-prefix/lib/node_modules/npm-global-prefix-pkg --assert missing`

```
custom-prefix/lib/node_modules/npm-global-prefix-pkg: missing
```

## `vpt stat-file custom-prefix/bin/npm-global-prefix-cli --assert missing`

```
custom-prefix/bin/npm-global-prefix-cli: missing
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert missing`

```
<home>/.vite-plus/bin/npm-global-prefix-cli: missing
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert missing`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: missing
```
