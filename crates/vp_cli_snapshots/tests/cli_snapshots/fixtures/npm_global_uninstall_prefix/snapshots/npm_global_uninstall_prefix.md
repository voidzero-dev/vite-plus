# npm_global_uninstall_prefix

## `vpt mkdir -p custom-prefix`

```
```

## `npm install -g --prefix ./custom-prefix ./npm-global-prefix-pkg`

Install to custom prefix, should create link

```

added 1 package in <duration>
Linked 'npm-global-prefix-cli' to <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli --assert symlink`

Link should exist

```
<home>/.vite-plus/bin/npm-global-prefix-cli: symlink
```

## `npm-global-prefix-cli`

Verify callable via link

```
npm-global-prefix-cli works
```

## `npm uninstall -g --prefix ./custom-prefix npm-global-prefix-pkg`

Uninstall should also remove link

```

removed 1 package in <duration>
Removed link 'npm-global-prefix-cli' from <home>/.vite-plus/bin/npm-global-prefix-cli
```

## `vpt stat-file $VP_HOME/bin/npm-global-prefix-cli`

Should be gone

```
<home>/.vite-plus/bin/npm-global-prefix-cli: missing
```

## `vpt rm -rf custom-prefix`

Cleanup

```
```
