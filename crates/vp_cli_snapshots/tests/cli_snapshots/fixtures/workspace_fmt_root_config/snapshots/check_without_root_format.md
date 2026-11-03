# check_without_root_format

## `vpt write-file ../../vite.config.ts 'export default { lint: {} };
'`


## `vpt write-file vite.config.ts 'export default { fmt: { singleQuote: true, semi: false } };
'`


## `vpt write-file index.js 'export const message = '\''hello'\''
'`


## `vp check --no-lint index.js`

Without a root fmt block, check lets Oxfmt discover the package format settings.

```
pass: All 1 file are correctly formatted (<duration>, <n> threads)
```
