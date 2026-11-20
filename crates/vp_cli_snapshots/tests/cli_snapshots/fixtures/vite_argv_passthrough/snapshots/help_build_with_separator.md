# help_build_with_separator

## `vp help build --`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp build [ROOT] [OPTIONS]

Build for production.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --target <target>             [string] transpile target (default: 'baseline-widely-available')
  --outDir <dir>                [string] output directory (default: dist)
  --assetsDir <dir>             [string] directory under outDir to place assets in (default: assets)
  --assetsInlineLimit <number>  [number] static asset base64 inline threshold in bytes (default: 4096)
  --ssr [entry]                 [string] build specified entry for server-side rendering
  --sourcemap [output]          [boolean | "inline" | "hidden"] output source maps for build (default: false)
  --minify [minifier]           [boolean | "oxc" | "terser" | "esbuild"] enable/disable minification, or specify minifier to use (default: oxc)
  --manifest [name]             [boolean | string] emit build manifest json
  --ssrManifest [name]          [boolean | string] emit ssr manifest json
  --emptyOutDir                 [boolean] force empty outDir when it's outside of root
  -w, --watch                   [boolean] rebuilds when modules have changed on disk
  --app                         [boolean] same as `builder: {}`
  --base <path>                 [string] public base path (default: /)
  -l, --logLevel <level>        [string] info | warn | error | silent
  --clearScreen                 [boolean] allow/disable clear screen when logging
  -d, --debug [feat]            [string | boolean] show debug logs
  -f, --filter <filter>         [string] filter debug logs
  -m, --mode <mode>             [string] set env mode
  -h, --help                    Display this message

Examples:
  vp build
  vp build --watch
  vp build --sourcemap

Documentation: https://viteplus.dev/guide/build
```

## `vpt stat-file vite-argv.json --assert missing`

```
vite-argv.json: missing
```

## `vpt stat-file dist/index.html --assert missing`

```
dist/index.html: missing
```
