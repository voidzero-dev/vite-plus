# optional_dependency_prompts_install

## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.0.0"}'`


## `vp lint src/index.js`

an optional vite-plus declaration should recommend installing dependencies

```
VITE+ - The Unified Toolchain for the Web

warn: No project-local vite-plus installation was found. Run `vp install` in `<workspace>/optional` to install dependencies.
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
