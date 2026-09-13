# nested_pnpm_preserves_inherited_npm

## `vpt write-file .node-version 22.18.0`


## `vpt json-edit package.json packageManager npm@10.5.0`


## `vp env on npm`


## `vp env on pnpm`


## `npx --version`


## `cd pnpm-child && pnpm --version`


## `npx --offline --call 'node assert-inherited-npm.cjs'`

A child project can add its pinned pnpm without replacing the parent's Node or npm/npx

```
Adding pnpm preserves inherited Node <version> and npm/npx 10.5.0
```
