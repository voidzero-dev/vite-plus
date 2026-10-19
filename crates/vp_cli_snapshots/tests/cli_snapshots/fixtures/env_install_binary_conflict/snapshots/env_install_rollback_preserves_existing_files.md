# env_install_rollback_preserves_existing_files

## `vpt json-edit env-binary-conflict-pkg-a/package.json bin '{"aaa-rollback-cli":"./cli.js","zzz-rollback-cli":"./cli.js"}'`

```
```

## `vpt write-file $VP_HOME/bin/aaa-rollback-cli '#'\!'/bin/sh
echo original-first
'`

```
```

## `vpt write-file $VP_HOME/bin/zzz-rollback-cli '#'\!'/bin/sh
echo original-unattempted
'`

```
```

## `vpt chmod +x $VP_HOME/bin/aaa-rollback-cli`

```
```

## `vpt chmod +x $VP_HOME/bin/zzz-rollback-cli`

```
```

## `vp install -g ./env-binary-conflict-pkg-a`

A regular file prevents shim creation without authorizing rollback to delete existing files

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
error: Failed to install env-binary-conflict-pkg-a: <home>/.vite-plus/bin/aaa-rollback-cli already exists and is not managed by Vite+
```

## `vpt stat-file $VP_HOME/bin/aaa-rollback-cli --assert file`

```
<home>/.vite-plus/bin/aaa-rollback-cli: file
```

## `aaa-rollback-cli`

```
original-first
```

## `vpt stat-file $VP_HOME/bin/zzz-rollback-cli --assert file`

```
<home>/.vite-plus/bin/zzz-rollback-cli: file
```

## `zzz-rollback-cli`

```
original-unattempted
```

## `vpt stat-file $VP_HOME/packages/env-binary-conflict-pkg-a.json --assert missing`

```
<home>/.vite-plus/packages/env-binary-conflict-pkg-a.json: missing
```

## `vpt stat-file $VP_HOME/bins/aaa-rollback-cli.json --assert missing`

```
<home>/.vite-plus/bins/aaa-rollback-cli.json: missing
```

## `vpt stat-file $VP_HOME/bins/zzz-rollback-cli.json --assert missing`

```
<home>/.vite-plus/bins/zzz-rollback-cli.json: missing
```
