# npm6_dependency_filters_outdated

## `vpt json-edit package.json packageManager npm@6.14.18`


## `vp install --ignore-scripts --silent`


## `NODE_ENV=production vp outdated -D`

npm 6 reports installed development dependencies

**Exit code:** 1

```
Package                     Current  Wanted  Latest  Location
test-vite-plus-top-package    1.0.0   1.0.0   1.1.0  command-outdated-npm
testnpm2                      1.0.0   1.0.0   1.0.1  command-outdated-npm
```

## `vp outdated -r -P --no-optional`

reject options without equivalent npm 6 options because npm 6 --production still reports installed devDependencies

**Exit code:** 1

```
npm < 7.14.0 does not support --recursive.
npm < 7 does not support --prod.
npm < 7 does not support --no-optional.
```
