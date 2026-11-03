# install_yarn_corepack_hash_mismatch

Every environment entry point must preserve and verify the same Corepack integrity suffix without repeating the expensive cache setup in separate cases.

## `vpt rm -rf $VP_HOME/package_manager/yarn/4.17.1 $VP_HOME/package_manager/yarn/4.17.1.lock`

Ensure the Corepack-pinned Yarn version is not cached


## `vp install`

Cache the verified Yarn CLI


## `vp env install pm`

The explicit environment install accepts the verified project hash

```
VITE+ - The Unified Toolchain for the Web

Installing yarn <version>...
Installed yarn <version>
```

## `vpt replace-file-content package.json b7ad4697 b7ad4698`

Change the pin to a hash that the cached CLI does not match


## `vp install`

The error names the artifact that the hash covers. vp does not download the CLI again

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Install error: Hash mismatch for yarn@4.17.1: expected sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4698, got sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4697
The `packageManager` hash covers the extracted Yarn CLI (bin/yarn.js). Corepack hashes the same artifact.
```

## `vp env install pm`

The explicit environment install rejects the mismatched project hash

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

Installing yarn <version>...
error: Install error: Hash mismatch for yarn@4.17.1: expected sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4698, got sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4697
The `packageManager` hash covers the extracted Yarn CLI (bin/yarn.js). Corepack hashes the same artifact.
```

## `CI=true vp env use pm --no-install`

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
