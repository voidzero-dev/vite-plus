# install_yarn_corepack_hash

## `vpt rm -rf $VP_HOME/package_manager/yarn/4.17.1 $VP_HOME/package_manager/yarn/4.17.1.lock`

Ensure the Corepack-pinned Yarn version is not cached


## `vpt stat-file $VP_HOME/package_manager/yarn/4.17.1 --assert missing`

Yarn 4.17.1 is not in the cache

```
<home>/.vite-plus/package_manager/yarn/<version>: missing
```

## `vp install`

A first install accepts the hash that Corepack wrote

```
VITE+ - The Unified Toolchain for the Web

➤ YN0000: · Yarn <version>
➤ YN0000: ┌ Resolution step
➤ YN0000: └ Completed
➤ YN0000: ┌ Fetch step
➤ YN0000: └ Completed
➤ YN0000: ┌ Link step
➤ YN0000: └ Completed
➤ YN0000: · Done in <duration> <duration>
```

## `vpt stat-file $VP_HOME/package_manager/yarn/4.17.1/yarn/bin/yarn.js --assert file`

The cache holds the Yarn CLI that vp verified

```
<home>/.vite-plus/package_manager/yarn/<version>/yarn/bin/yarn.js: file
```
