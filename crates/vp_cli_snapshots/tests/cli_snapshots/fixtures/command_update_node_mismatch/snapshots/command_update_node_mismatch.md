# command_update_node_mismatch

## `vp install -g --node 20 testnpm2`

```
info: Installing 1 global package with Node.js <version>
✓ Installed testnpm2 1.0.1
```

## `vp update -g testnpm2`

should warn and skip node mismatch reinstall in CI

```
All global packages are up to date.
warn: Skipping reinstall for global packages installed with a different Node.js version: testnpm2. Use --reinstall-node-mismatch to reinstall them.
```

## `vp update -g testnpm2 --ignore-node-mismatch`

should explicitly skip node mismatch reinstall

```
All global packages are up to date.
```

## `vp update -g testnpm2 --reinstall-node-mismatch`

```
info: Updating 1 global package with Node.js <version>
✓ Updated testnpm2 to 1.0.1
```
