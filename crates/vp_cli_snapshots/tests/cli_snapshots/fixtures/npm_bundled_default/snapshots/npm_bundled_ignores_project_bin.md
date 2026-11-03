# npm_bundled_ignores_project_bin

## `vpt write-file bin/npm '#'\!'/bin/sh
echo project npm must not run
exit 1
'`


## `vpt chmod +x bin/npm`


## `node assert-npm.cjs`

The version probe and actual command both use bundled npm despite an executable project bin/npm

```
npm/npx and vp use Node bundled npm
```
