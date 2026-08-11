# run_yarn_corepack_hash_mismatch

## `vpt rm -rf $VP_HOME/package_manager/yarn/4.17.1 $VP_HOME/package_manager/yarn/4.17.1.lock`

Ensure the Corepack-pinned Yarn version is not cached


## `vp install`

Cache the verified Yarn CLI


## `vpt replace-file-content package.json b7ad4697 b7ad4698`

Change the pin to a hash that the cached CLI does not match


## `vp run smoke`

vp run reports the integrity failure. It does not start the task

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

error: Hash mismatch for yarn@4.17.1: expected sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4698, got sha512.ccbfabf7d7b6b32075088be9386fb9a2e00bb6887ef07fa56effabc890a56d53da1ccc4128d62db245fcbd3961b236d75335bdf7d5320ed6eafb7588b7ad4697
The `packageManager` hash covers the extracted Yarn CLI (bin/yarn.js). Corepack hashes the same artifact.
```
