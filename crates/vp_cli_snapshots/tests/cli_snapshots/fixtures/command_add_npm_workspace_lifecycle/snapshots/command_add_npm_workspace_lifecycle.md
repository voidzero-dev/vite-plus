# command_add_npm_workspace_lifecycle

## `npm pack ./local-package --pack-destination packages/app --silent`


## `vpt mkdir packages/app/test-global`


## `cd packages/app && NPM_CONFIG_PREFIX=${workspace}/test-global vp add -w ./lifecycle-probe-1.0.0.tgz --offline -- --no-audit --no-fund`


## `node check.cjs`

```
Root dependencies resolve and the install script retains its caller and global prefix.
```
