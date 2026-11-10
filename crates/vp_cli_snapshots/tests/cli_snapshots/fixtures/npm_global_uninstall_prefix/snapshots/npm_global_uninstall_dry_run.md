# npm_global_uninstall_dry_run

## `npm install -g --prefix ./custom-prefix ./npm-global-prefix-pkg`

Install a local package and create its Vite+ link

```

added 1 package in <duration>
Linked 'npm-global-prefix-cli' to <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `npm uninstall -g --prefix ./custom-prefix npm-global-prefix-pkg --dry-run`

CLI dry-run must preserve the package, npm binary, link, and BinConfig

```
remove npm-global-prefix-pkg 1.0.0

removed 1 package in <duration>
```

## `vpt stat-file custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json --assert file`

```
custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json: file
```

## `vpt stat-file custom-prefix/bin/npm-global-prefix-cli --assert symlink`

```
custom-prefix/bin/npm-global-prefix-cli: symlink
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

## `npm_config_dry_run=true npm uninstall -g --prefix ./custom-prefix npm-global-prefix-pkg`

Environment-configured dry-run must also preserve installed state

```
remove npm-global-prefix-pkg 1.0.0

removed 1 package in <duration>
```

## `vpt stat-file custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json --assert file`

```
custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json: file
```

## `vpt stat-file custom-prefix/bin/npm-global-prefix-cli --assert symlink`

```
custom-prefix/bin/npm-global-prefix-cli: symlink
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

## `vpt write-file dry-run.npmrc 'dry-run=true
'`

```
```

## `npm_config_userconfig=${workspace}/dry-run.npmrc npm uninstall -g --prefix ./custom-prefix npm-global-prefix-pkg`

User npmrc dry-run must also preserve installed state

```
remove npm-global-prefix-pkg 1.0.0

removed 1 package in <duration>
```

## `vpt stat-file custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json --assert file`

```
custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json: file
```

## `vpt stat-file custom-prefix/bin/npm-global-prefix-cli --assert symlink`

```
custom-prefix/bin/npm-global-prefix-cli: symlink
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

## `npm uninstall -g --prefix ./custom-prefix npm-global-prefix-pkg`

A real uninstall must still clean up the package, npm binary, link, and BinConfig

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
