# install_package_manager_default

An unconfigured project installs with pnpm without prompting or pinning the manager.

## `vp install --lockfile-only`

```

Done in <duration> using pnpm <version>
```

## `vpt stat-file pnpm-lock.yaml --assert file`

```
pnpm-lock.yaml: file
```

## `vpt print-file package.json`

```
{"name":"install-package-manager-default","private":true}
```
