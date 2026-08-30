# non_interactive_use_preserves_system_without_deciding

## `vpt rm -f $VP_HOME/config.json`


## `vpt chmod +x system-bin/pnpm`


## `PATH=${VP_HOME}/bin${PATH_SEPARATOR}${workspace}/system-bin${PATH_SEPARATOR}${PATH} pnpm --version`

a non-interactive upgrade keeps using system pnpm without prompting

```
system-pnpm
```

## `vpt stat-file $VP_HOME/config.json --assert missing`

non-interactive use does not record a choice

```
<home>/.vite-plus/config.json: missing
```
