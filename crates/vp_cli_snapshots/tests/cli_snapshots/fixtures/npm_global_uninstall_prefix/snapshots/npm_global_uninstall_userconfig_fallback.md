# npm_global_uninstall_userconfig_fallback

## `node private-runtime.mjs`

```
```

## `npm install -g ./npm-global-prefix-pkg`

Install into a private copy of the selected Node runtime

```

added 1 package in <duration>
Linked 'npm-global-prefix-cli' to <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file node-prefix/lib/node_modules/npm-global-prefix-pkg/package.json --assert file`

```
node-prefix/lib/node_modules/npm-global-prefix-pkg/package.json: file
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert symlink`

```
<home>/.vite-plus/bin/npm-global-prefix-cli: symlink
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert file`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: file
```

## `npm config set prefix ./other-prefix --location=user`

Make the normal user config point to another prefix

```
```

## `vpt write-file empty.npmrc `

```
```

## `npm uninstall -g --userconfig=empty.npmrc npm-global-prefix-pkg --dry-run`

A config override dry-run must preserve the fallback installation

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

## `npm uninstall -g npm-global-prefix-pkg`

Uninstalling from an empty prefix must not remove links to the surviving fallback package

```

up to date in <duration>
```

## `npm-global-prefix-cli`

```
npm-global-prefix-cli works
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert file`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: file
```

## `npm uninstall -g --userconfig empty.npmrc npm-global-prefix-pkg`

A real uninstall using the override must clean up the fallback package's links

```

removed 1 package in <duration>
Removed link 'npm-global-prefix-cli' from <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file node-prefix/lib/node_modules/npm-global-prefix-pkg --assert missing`

```
node-prefix/lib/node_modules/npm-global-prefix-pkg: missing
```

## `vpt stat-file node-prefix/bin/npm-global-prefix-cli --assert missing`

```
node-prefix/bin/npm-global-prefix-cli: missing
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert missing`

```
<home>/.vite-plus/bin/npm-global-prefix-cli: missing
```

## `vpt stat-file $VP_HOME/bins/npm-global-prefix-cli.json --assert missing`

```
<home>/.vite-plus/bins/npm-global-prefix-cli.json: missing
```
