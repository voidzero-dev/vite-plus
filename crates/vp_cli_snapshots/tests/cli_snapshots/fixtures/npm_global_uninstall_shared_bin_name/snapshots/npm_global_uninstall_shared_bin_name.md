# npm_global_uninstall_shared_bin_name

## `npm install -g ./pkg-a`

Install pkg-a (creates link for npm-global-shared-cli)

```

added 1 package in <duration>
Linked 'npm-global-shared-cli' to <home>/.vite-plus/bin/npm-global-shared-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-shared-cli --assert symlink`

```
<home>/.vite-plus/bin/npm-global-shared-cli: symlink
```

## `vpt print-file $VP_HOME/bins/npm-global-shared-cli.json`

BinConfig should point to pkg-a

```
{
  "name": "npm-global-shared-cli",
  "package": "npm-global-shared-pkg-a",
  "version": "",
  "nodeVersion": "<version>",
  "source": "npm"
}
```

## `npm install -g --force ./pkg-b`

Install pkg-b with force (overwrites npm-global-shared-cli)

```

added 1 package in <duration>
npm warn using --force Recommended protections disabled.
Linked 'npm-global-shared-cli' to <home>/.vite-plus/bin/npm-global-shared-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-shared-cli --assert symlink`

```
<home>/.vite-plus/bin/npm-global-shared-cli: symlink
```

## `vpt print-file $VP_HOME/bins/npm-global-shared-cli.json`

BinConfig should now point to pkg-b

```
{
  "name": "npm-global-shared-cli",
  "package": "npm-global-shared-pkg-b",
  "version": "",
  "nodeVersion": "<version>",
  "source": "npm"
}
```

## `npm-global-shared-cli`

Should print pkg-b message (latest installed)

```
shared-cli from pkg-b
```

## `npm uninstall -g npm-global-shared-pkg-a`

Uninstall pkg-a, should NOT remove the link (owned by pkg-b now)

```

removed 1 package in <duration>
Linked 'npm-global-shared-cli' to <home>/.vite-plus/bin/npm-global-shared-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-shared-cli`

link should still exist

```
<home>/.vite-plus/bin/npm-global-shared-cli: symlink
```

## `npm-global-shared-cli`

Should still work (owned by pkg-b)

```
shared-cli from pkg-b
```

## `npm uninstall -g npm-global-shared-pkg-b`

Uninstall pkg-b, NOW should remove the link

```

removed 1 package in <duration>
Removed link 'npm-global-shared-cli' from <home>/.vite-plus/bin/npm-global-shared-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-shared-cli`

link should be removed

```
<home>/.vite-plus/bin/npm-global-shared-cli: missing
```
