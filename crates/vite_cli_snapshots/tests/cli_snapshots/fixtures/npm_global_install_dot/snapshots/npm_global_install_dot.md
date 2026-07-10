# npm_global_install_dot

## `cd npm-global-dot-pkg && npm install -g .`

Should install and create link

```

added 1 package in <duration>
Linked 'npm-global-dot-cli' to <home>/.vite-plus/bin/npm-global-dot-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-dot-cli --assert symlink`

Link should exist

```
<home>/.vite-plus/bin/npm-global-dot-cli: symlink
```

## `npm-global-dot-cli`

Should be callable via the link

```
npm-global-dot-cli works
```

## `vpt rm -f $VP_HOME/bin/npm-global-dot-cli`

Cleanup link

```
```

## `npm uninstall -g npm-global-dot-pkg`

Cleanup npm install

```

removed 1 package in <duration>
```
