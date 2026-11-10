# npm_global_uninstall_preserves_local_copy

## `vpt mkdir -p custom-prefix`

```
```

## `vpt write-file custom-prefix/package.json '{"private":true}'`

```
```

## `npm install --prefix ./custom-prefix --ignore-scripts --no-save --no-package-lock --no-audit ./npm-global-prefix-pkg`

Install a local copy under the same prefix

```

added 1 package in <duration>
```

## `npm install -g --prefix ./custom-prefix ./npm-global-prefix-pkg`

Install a separate global copy and create its Vite+ link

```

added 1 package in <duration>
Linked 'npm-global-prefix-cli' to <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file custom-prefix/node_modules/npm-global-prefix-pkg/package.json --assert file`

```
custom-prefix/node_modules/npm-global-prefix-pkg/package.json: file
```

## `vpt stat-file custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json --assert file`

```
custom-prefix/lib/node_modules/npm-global-prefix-pkg/package.json: file
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert symlink`

```
<home>/.vite-plus/bin/npm-global-prefix-cli: symlink
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert file`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: file
```

## `npm uninstall -g --prefix ./custom-prefix npm-global-prefix-pkg --dry-run`

Dry-run must preserve the global command even when a local copy also exists

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

## `npm uninstall -g --prefix ./custom-prefix npm-global-prefix-pkg`

Real uninstall must remove the global links without confusing the remaining local copy for the global installation

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

## `vpt stat-file custom-prefix/node_modules/npm-global-prefix-pkg/package.json --assert file`

```
custom-prefix/node_modules/npm-global-prefix-pkg/package.json: file
```

## `node custom-prefix/node_modules/npm-global-prefix-pkg/cli.js`

```
npm-global-prefix-cli works
```
