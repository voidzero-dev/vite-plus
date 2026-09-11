# env_install_binary_conflict

## `vp install -g ./env-binary-conflict-pkg-a`

Install pkg-a which provides env-binary-conflict-cli binary

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
✓ Installed env-binary-conflict-pkg-a 1.0.0
  Bins: env-binary-conflict-cli
```

## `vpt print-file $VP_HOME/bins/env-binary-conflict-cli.json`

Bin config should point to pkg-a

```
{
  "name": "env-binary-conflict-cli",
  "package": "env-binary-conflict-pkg-a",
  "version": "1.0.0",
  "nodeVersion": "<version>",
  "source": "vp"
}
```

## `vp install -g ./env-binary-conflict-pkg-b`

Try to install pkg-b without force - should fail

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
error: Failed to install env-binary-conflict-pkg-b: Executable 'env-binary-conflict-cli' is already installed by env-binary-conflict-pkg-a

Please remove env-binary-conflict-pkg-a before installing env-binary-conflict-pkg-b, or use --force to auto-replace
```

## `vpt print-file $VP_HOME/bins/env-binary-conflict-cli.json`

Bin config should still point to pkg-a

```
{
  "name": "env-binary-conflict-cli",
  "package": "env-binary-conflict-pkg-a",
  "version": "1.0.0",
  "nodeVersion": "<version>",
  "source": "vp"
}
```

## `vp install -g --force ./env-binary-conflict-pkg-b`

Force install pkg-b - should auto-uninstall pkg-a

```
VITE+ - The Unified Toolchain for the Web

info: Installing 1 global package with Node.js <version>
Uninstalling env-binary-conflict-pkg-a (conflicts with env-binary-conflict-pkg-b)...
Uninstalled env-binary-conflict-pkg-a
✓ Installed env-binary-conflict-pkg-b 2.0.0
  Bins: env-binary-conflict-cli
```

## `vpt print-file $VP_HOME/bins/env-binary-conflict-cli.json`

Bin config should now point to pkg-b

```
{
  "name": "env-binary-conflict-cli",
  "package": "env-binary-conflict-pkg-b",
  "version": "2.0.0",
  "nodeVersion": "<version>",
  "source": "vp"
}
```

## `vp remove -g env-binary-conflict-pkg-b`

Cleanup

```
Uninstalled env-binary-conflict-pkg-b
```

## `vpt stat-file $VP_HOME/bins/env-binary-conflict-cli.json --assert missing`

Bin config should be deleted

```
<home>/.vite-plus/bins/env-binary-conflict-cli.json: missing
```
