# env_package_manager_hash

## `vpt rm -rf $VP_HOME/package_manager/yarn/4.17.1 $VP_HOME/package_manager/yarn/4.17.1.lock`

Ensure the Corepack-pinned Yarn version is not cached


## `vp env install pm`

The explicit environment install accepts and verifies the project hash

```
VITE+ - The Unified Toolchain for the Web

Installing yarn <version>...
Installed yarn <version>
```

## `vpt replace-file-content package.json b7ad4697 b7ad4698`

Change the project pin to a hash that the cached CLI does not match


## `vp env install pm`

The explicit environment install rejects the mismatched project hash

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Installing yarn <version>...
error: Install error: Hash mismatch for yarn@4.17.1: expected sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4698, got sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4697
The `packageManager` hash covers the extracted Yarn CLI (bin/yarn.js). Corepack hashes the same artifact.
```

## `vp env use pm --no-install`

The session override preserves the project hash for first use

```
Using yarn <version> (resolved from packageManager)
```

## `yarn --version`

The package-manager shim enforces the hash retained by env use

**Exit code:** 1

```
vp: Failed to resolve package manager for 'yarn': Install error: Hash mismatch for yarn@4.17.1: expected sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4698, got sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4697
The `packageManager` hash covers the extracted Yarn CLI (bin/yarn.js). Corepack hashes the same artifact.
```

## `vp env use pm --unset`
