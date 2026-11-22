# pack_array_helper_with_imported_concat

Keep the library mutation and resources when an imported array concatenates the pack options.

## `vpt json-edit package.json devDependencies.vite-plus ^1.7.0`


## `vpt json-edit package.json devDependencies.tsdown 0.22.14`


## `vpt json-edit package.json scripts.bundle 'tsdown -c tsdown.config.ts'`


## `vpt write-file tsdown.config.ts 'export default { entry: ['\''src/library.ts'\''], outDir: '\''library-out'\'', publicDir: '\''public'\'' };
'`


## `vpt write-file app-builds.ts 'export default [{ entry: ['\''src/browser.ts'\''], outDir: '\''apps-out'\'' }];
'`


## `vpt write-file src/cli.ts 'export const cli = true;
'`


## `vpt write-file src/browser.ts 'export const browser = true;
'`


## `vpt write-file src/extra.ts 'export const extra = true;
'`


## `vpt write-file vite.config.ts 'import apps from '\''./app-builds.ts'\'';
import library from '\''./tsdown.config.ts'\'';
const cli = { entry: ['\''src/cli.ts'\''], outDir: '\''cli-out'\'' };
function configure(builds: Array<typeof cli>) {
  builds[0].entry.push('\''src/extra.ts'\'');
  builds[0].outDir = '\''release-out'\'';
}
configure([library, cli]);
export default { pack: apps.concat(library, cli) };
'`


## `vpt mkdir -p public`


## `vpt write-file public/resource.txt 'library resource
'`


## `vp install --ignore-scripts`


## `vp migrate --full --no-interactive --no-hooks --no-agent --no-editor`


## `vp pack`


## `vpt stat-file release-out/library.mjs release-out/extra.mjs cli-out/cli.mjs apps-out/browser.mjs --assert file`

```
release-out/library.mjs: file
release-out/extra.mjs: file
cli-out/cli.mjs: file
apps-out/browser.mjs: file
```

## `vpt print-file release-out/public/resource.txt`

```
library resource
```

## `vpt stat-file library-out --assert missing`

```
library-out: missing
```

## `pnpm run bundle`


## `vpt stat-file library-out/library.mjs --assert file`

```
library-out/library.mjs: file
```

## `vpt stat-file library-out/extra.mjs --assert missing`

```
library-out/extra.mjs: missing
```

## `vpt print-file library-out/public/resource.txt`

```
library resource
```
