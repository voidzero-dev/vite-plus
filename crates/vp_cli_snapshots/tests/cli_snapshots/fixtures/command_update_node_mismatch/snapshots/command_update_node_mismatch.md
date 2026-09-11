# command_update_node_mismatch

## `vp install -g --node 20 testnpm2`

```
[1m[94minfo:[39m[0m Installing 1 global package with Node.js <version>
[32m✓[39m Installed [1mtestnpm2[0m [1m1.0.1[0m
```

## `vp update -g testnpm2`

should warn and skip node mismatch reinstall in CI

```
All global packages are up to date.
[1m[33mwarn:[39m[0m Skipping reinstall for global packages installed with a different Node.js version: testnpm2. Use --reinstall-node-mismatch to reinstall them.
```

## `vp update -g testnpm2 --ignore-node-mismatch`

should explicitly skip node mismatch reinstall

```
All global packages are up to date.
```

## `vp update -g testnpm2 --reinstall-node-mismatch`

```
[1m[94minfo:[39m[0m Updating 1 global package with Node.js <version>
[32m✓[39m Updated [1mtestnpm2[0m to [1m1.0.1[0m
```
