# chdir_install_guidance_names_target

## `vpt write-file ../target/node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.0.0"}'`


## `vp -C ../target lint src/index.js`

install guidance should name the project selected by -C

```
VITE+ - The Unified Toolchain for the Web

warn: No project-local vite-plus installation was found. Run `vp install` in `<workspace>/target` to install dependencies.
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```
