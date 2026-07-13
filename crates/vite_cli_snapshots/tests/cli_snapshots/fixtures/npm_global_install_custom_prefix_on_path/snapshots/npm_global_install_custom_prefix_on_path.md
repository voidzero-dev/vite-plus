# npm_global_install_custom_prefix_on_path

## `vpt mkdir -p custom-prefix-on-path/bin`


## `PATH=${workspace}/custom-prefix-on-path/bin:${PATH} NPM_CONFIG_PREFIX=${workspace}/custom-prefix-on-path npm install -g ./npm-global-on-path-pkg`

Should install without hint (bin dir on PATH)

```

added 1 package in <duration>
```

## `vpt stat-file custom-prefix-on-path/bin/npm-global-on-path-cli --assert symlink`

Verify installed to custom prefix

```
custom-prefix-on-path/bin/npm-global-on-path-cli: symlink
```

## `vpt stat-file $VP_HOME/bin/npm-global-on-path-cli --assert missing`

No link should be created

```
<home>/.vite-plus/bin/npm-global-on-path-cli: missing
```
