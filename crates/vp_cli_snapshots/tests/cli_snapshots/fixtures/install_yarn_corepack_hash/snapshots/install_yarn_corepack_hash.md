# install_yarn_corepack_hash

## `vpt rm -rf $VP_HOME/package_manager/yarn/4.17.1 $VP_HOME/package_manager/yarn/.verified-v1/4.17.1 $VP_HOME/package_manager/yarn/4.17.1.lock`

Ensure the Corepack-pinned Yarn version is not cached


## `vpt stat-file $VP_HOME/package_manager/yarn/.verified-v1/4.17.1 --assert missing`

Yarn 4.17.1 is not in the cache

```
<home>/.vite-plus/package_manager/yarn/.verified-v1/<version>: missing
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
➤ YN0000: · Done in <duration>
```

## `vpt stat-file $VP_HOME/package_manager/yarn/.verified-v1/4.17.1/3156b767c366db0f2c765c40a66f4ace/yarn/bin/yarn.js --assert file`

The cache holds the Yarn CLI that vp verified, under the content key of the pinned hash (short_fingerprint of the packageManager hash)

```
<home>/.vite-plus/package_manager/yarn/.verified-v1/<version>/<key>/yarn/bin/yarn.js: file
```
