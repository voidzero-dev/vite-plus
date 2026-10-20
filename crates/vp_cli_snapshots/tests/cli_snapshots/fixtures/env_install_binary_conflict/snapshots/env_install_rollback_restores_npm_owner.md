# env_install_rollback_restores_npm_owner

## `npm install -g ./env-binary-conflict-pkg-a`

```

added 1 package in <duration>
Linked 'env-binary-conflict-cli' to <home>/.vite-plus/bin/env-binary-conflict-cli
```

## `vpt print-file $VP_HOME/bins/env-binary-conflict-cli.json`

```
{
  "name": "env-binary-conflict-cli",
  "package": "env-binary-conflict-pkg-a",
  "version": "",
  "nodeVersion": "<version>",
  "source": "npm"
}
```

## `vpt json-edit env-binary-conflict-pkg-a/package.json bin.zzz-rollback-cli ./cli.js`

```
```

## `vpt mkdir $VP_HOME/bin/zzz-rollback-cli`

```
```

## `vp install -g ./env-binary-conflict-pkg-a`

A failure after replacing the npm link must restore the link and its ownership

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
error: Failed to install env-binary-conflict-pkg-a: Cannot replace non-file <home>/.vite-plus/bin/zzz-rollback-cli
```

## `vpt stat-file $VP_HOME/bin/env-binary-conflict-cli --assert symlink`

```
<home>/.vite-plus/bin/env-binary-conflict-cli: symlink
```

## `env-binary-conflict-cli`

```
Hello from pkg-a!
```

## `vpt print-file $VP_HOME/bins/env-binary-conflict-cli.json`

```
{
  "name": "env-binary-conflict-cli",
  "package": "env-binary-conflict-pkg-a",
  "version": "",
  "nodeVersion": "<version>",
  "source": "npm"
}
```

## `vpt stat-file $VP_HOME/bin/zzz-rollback-cli --assert dir`

```
<home>/.vite-plus/bin/zzz-rollback-cli: dir
```

## `vpt stat-file $VP_HOME/packages/env-binary-conflict-pkg-a.json --assert missing`

```
<home>/.vite-plus/packages/env-binary-conflict-pkg-a.json: missing
```

## `vpt stat-file $VP_HOME/bins/zzz-rollback-cli.json --assert missing`

```
<home>/.vite-plus/bins/zzz-rollback-cli.json: missing
```
