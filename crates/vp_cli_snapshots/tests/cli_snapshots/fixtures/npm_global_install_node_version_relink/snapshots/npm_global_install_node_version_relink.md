# npm_global_install_node_version_relink

## `NPM_CONFIG_PREFIX=${workspace}/node-24 npm install -g --ignore-scripts ./pkg`

Install with Node.js 24 (creates the link)

```

added 1 package in <duration>
Linked 'npm-global-relink-cli' to <home>/.vite-plus/bin/npm-global-relink-cli
```

## `vpt grep-file $VP_HOME/bins/npm-global-relink-cli.json '"nodeVersion": "24.22.1"'`

BinConfig records Node.js 24

```
<home>/.vite-plus/bins/npm-global-relink-cli.json: found "\"nodeVersion\": \"24.22.1\""
```

## `VP_NODE_VERSION=26.12.1 NPM_CONFIG_PREFIX=${workspace}/node-26 npm install -g --ignore-scripts ./pkg`

Install again with Node.js 26 (moves the link to the new prefix)

```

added 1 package in <duration>
note: Moving 'npm-global-relink-cli' from Node.js <version> to 26.12.1
Linked 'npm-global-relink-cli' to <home>/.vite-plus/bin/npm-global-relink-cli
```

## `vpt grep-file $VP_HOME/bins/npm-global-relink-cli.json '"nodeVersion": "26.12.1"'`

BinConfig records Node.js 26

```
<home>/.vite-plus/bins/npm-global-relink-cli.json: found "\"nodeVersion\": \"26.12.1\""
```

## `NPM_CONFIG_PREFIX=${workspace}/node-24 npm install -g --ignore-scripts --dry-run ./pkg`

Dry run with Node.js 24, whose prefix still has the package (the link stays on Node.js 26)

```

up to date in <duration>
```

## `NPM_CONFIG_PREFIX=${workspace}/node-24 npm_config_dry_run=true npm install -g --ignore-scripts ./pkg`

Dry run from the npm environment config (the link stays on Node.js 26)

```

up to date in <duration>
```

## `vpt grep-file $VP_HOME/bins/npm-global-relink-cli.json '"nodeVersion": "26.12.1"'`

BinConfig still records Node.js 26

```
<home>/.vite-plus/bins/npm-global-relink-cli.json: found "\"nodeVersion\": \"26.12.1\""
```

## `vpt rm -rf node-24`

Remove the Node.js 24 copy: the command must not depend on it

```
```

## `npm-global-relink-cli`

Should be callable via the moved link

```
relink-cli works
```

## `VP_NODE_VERSION=26.12.1 NPM_CONFIG_PREFIX=${workspace}/node-26 npm install -g --ignore-scripts ./pkg`

Install again with Node.js 26 (the link is current, so no message)

```

up to date in <duration>
```
