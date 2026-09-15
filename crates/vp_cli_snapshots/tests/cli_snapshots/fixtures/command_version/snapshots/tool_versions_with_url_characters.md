# tool_versions_with_url_characters

Tool versions load from filesystem paths with spaces and URL fragment characters.

## `vpt mkdir -p 'project #1/node_modules/vite-plus/dist'`


## `vpt write-file 'project #1/package.json' '{"name":"project","devDependencies":{"vite-plus":"9.8.7"}}'`


## `vpt write-file 'project #1/node_modules/vite-plus/package.json' '{"name":"vite-plus","version":"9.8.7","type":"module"}'`


## `vpt write-file 'project #1/node_modules/vite-plus/dist/versions.js' 'export const versions = { vite: "9.8.7", rolldown: "9.8.7", vitest: "9.8.7", oxfmt: "9.8.7", oxlint: "9.8.7", "oxlint-tsgolint": "9.8.7", tsdown: "9.8.7" };'`


## `cd 'project #1' && vp --version`

```
VITE+ - The Unified Toolchain for the Web

vp <version>

Local vite-plus:
  vite-plus  <version>

Tools:
  vite             <version>
  rolldown         <version>
  vitest           <version>
  oxfmt            <version>
  oxlint           <version>
  oxlint-tsgolint  <version>
  tsdown           <version>
```
