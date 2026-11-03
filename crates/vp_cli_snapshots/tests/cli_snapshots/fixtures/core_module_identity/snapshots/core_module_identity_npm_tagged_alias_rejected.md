# core_module_identity_npm_tagged_alias_rejected

## `vpt write-file package.json '{"name":"core-module-identity","private":true,"type":"module","packageManager":"npm@11.11.0","devDependencies":{"vite-plus":"latest","vite":"npm:@voidzero-dev/vite-plus-core@latest"}}
'`


## `vp install --ignore-scripts`


## `node check-tagged-alias.mjs`

```
vp dev rejects the replaced npm alias and reports the repair command
vp pack rejects the replaced npm alias and reports the repair command
```
