# npm_global_install_hint

## `npm install -g ./npm-global-hint-pkg`

Should install and create link

```

added 1 package in <duration>
Linked 'npm-global-hint-cli' to <home>/.vite-plus/bin/npm-global-hint-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-hint-cli --assert symlink`

Link should exist

```
<home>/.vite-plus/bin/npm-global-hint-cli: symlink
```

## `vp env which npm-global-hint-cli`

Should report npm-created link

```
VITE+ - The Unified Toolchain for the Web

<workspace>/npm-global-hint-pkg/cli.js
  Package:    npm-global-hint-pkg
  Source:     npm
  Node:       <version>
```

## `npm-global-hint-cli`

Should be callable via the link

```
npm-global-hint-cli works
```

## `vpt rm -f $VP_HOME/bin/npm-global-hint-cli`

Cleanup link

```
```

## `npm uninstall -g npm-global-hint-pkg`

Cleanup npm install

```

removed 1 package in <duration>
```
