# npm_global_uninstall_link_cleanup

## `npm install -g ./npm-global-uninstall-pkg`

Install package first

```

added 1 package in <duration>
Linked 'npm-global-uninstall-cli' to <home>/.vite-plus/bin/npm-global-uninstall-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-uninstall-cli --assert symlink`

Link should exist after install

```
<home>/.vite-plus/bin/npm-global-uninstall-cli: symlink
```

## `npm uninstall -g npm-global-uninstall-pkg`

Uninstall should remove the link

```

removed 1 package in <duration>
Removed link 'npm-global-uninstall-cli' from <home>/.vite-plus/bin/npm-global-uninstall-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-uninstall-cli --assert-not symlink`

Link should be gone

```
<home>/.vite-plus/bin/npm-global-uninstall-cli: missing
```
