# env_install_without_backup_storage

## `vpt mkdir readonly-tmp`

```
```

## `vpt chmod 555 readonly-tmp`

```
```

## `vpt json-edit env-binary-conflict-pkg-a/package.json bin {}`

```
```

## `TMPDIR=${workspace}/readonly-tmp vp install -g ./env-binary-conflict-pkg-a`

A package without binaries needs no rollback storage

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
 Installed env-binary-conflict-pkg-a 1.0.0
```

## `vpt stat-file $VP_HOME/packages/env-binary-conflict-pkg-a.json --assert file`

```
<home>/.vite-plus/packages/env-binary-conflict-pkg-a.json: file
```

## `TMPDIR=${workspace}/readonly-tmp vp install -g ./env-binary-conflict-pkg-b`

New binaries need no backup of existing files

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
 Installed env-binary-conflict-pkg-b 2.0.0
  Bins: env-binary-conflict-cli
```

## `env-binary-conflict-cli`

```
Hello from pkg-b!
```

## `vpt stat-file $VP_HOME/bins/env-binary-conflict-cli.json --assert file`

```
<home>/.vite-plus/bins/env-binary-conflict-cli.json: file
```
