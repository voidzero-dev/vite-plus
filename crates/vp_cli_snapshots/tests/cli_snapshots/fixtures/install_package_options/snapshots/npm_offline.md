# npm_offline

## `npm --version`

provision npm before pointing its registry at an unreachable endpoint


## `npm_config_registry=http://127.0.0.1:9 vp install vp-install-option-uncached-probe@1.0.0 --offline`

an uncached package fails in offline mode rather than attempting a connection

**Exit code:** 1

```
VITE+ - The Unified Toolchain for the Web

npm error code ENOTCACHED
npm error request to http://127.0.0.1:<port>/vp-install-option-uncached-probe failed: cache mode is 'only-if-cached' but no cached response is available.
npm error A complete log of this run can be found in: <workspace>/.npm-cache/_logs/<timestamp>-debug-0.log
```

## `vpt stat-file node_modules --assert missing`

```
node_modules: missing
```

## `vpt stat-file package-lock.json --assert missing`

```
package-lock.json: missing
```
