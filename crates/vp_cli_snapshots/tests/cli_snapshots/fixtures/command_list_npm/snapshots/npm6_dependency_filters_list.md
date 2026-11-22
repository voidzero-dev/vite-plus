# npm6_dependency_filters_list

## `vpt json-edit package.json packageManager npm@6.14.18`


## `vpt json-edit package.json peerDependencies {}`


## `vp install --ignore-scripts --silent`


## `NODE_ENV=production vp pm list -D --json`

npm 6 lists both production and development dependencies

```
{
  "name": "command-list-npm",
  "version": "1.0.0",
  "dependencies": {
    "test-vite-plus-package": {
      "version": "1.0.0",
      "from": "test-vite-plus-package@1.0.0",
      "resolved": "http://127.0.0.1:<port>/test-vite-plus-package/-/test-vite-plus-package-1.0.0.tgz"
    },
    "testnpm2": {
      "version": "1.0.1",
      "from": "testnpm2@1.0.1",
      "resolved": "http://127.0.0.1:<port>/testnpm2/-/testnpm2-1.0.1.tgz"
    }
  }
}
```

## `vp pm list -P --json`

npm 6 uses --production to list only production dependencies

```
{
  "name": "command-list-npm",
  "version": "1.0.0",
  "dependencies": {
    "testnpm2": {
      "version": "1.0.1",
      "from": "testnpm2@1.0.1",
      "resolved": "http://127.0.0.1:<port>/testnpm2/-/testnpm2-1.0.1.tgz"
    }
  }
}
```

## `vp pm list --no-optional --exclude-peers`

reject omit filters without equivalent npm 6 options

**Exit code:** 1

```
npm < 7 does not support --no-optional.
npm < 7 does not support --exclude-peers.
```
