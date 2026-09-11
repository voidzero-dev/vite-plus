# missing_local_cli_warning

## `vpt write-file node_modules/vite-plus/package.json '{"name":"vite-plus","version":"0.0.0"}'`


## `vp lint src/index.js`

a project that does not declare vite-plus gets migration guidance

```
VITE+ - The Unified Toolchain for the Web

warn: This project does not use vite-plus. Learn how to migrate: https://viteplus.dev/guide/migrate
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt json-edit package.json devDependencies.vite-plus 0.0.0`


## `vp lint src/index.js`

a project that declares vite-plus but has no local CLI gets installation guidance

```
VITE+ - The Unified Toolchain for the Web

warn: No project-local vite-plus installation was found. Run `vp install` in `<workspace>` to install dependencies.
Found 0 warnings and 0 errors.
Finished in <duration> on 1 file with <n> rules using <n> threads.
```

## `vpt rm package.json`


## `node assert-silent-fallback.mjs`

outside a project, global fallback remains silent

```
Global fallback remained silent.
```
