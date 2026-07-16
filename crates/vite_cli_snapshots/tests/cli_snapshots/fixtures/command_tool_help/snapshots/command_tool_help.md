# command_tool_help

Tool-backed command help is rendered by the local vite-plus CLI.

## `vp dev --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp dev [ROOT] [OPTIONS]

Run the development server.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --host [HOST]            Specify hostname
  --port <PORT>            Specify port
  --open [PATH]            Open browser on startup
  --cors                   Enable CORS
  --strictPort             Exit if specified port is already in use
  --force                  Ignore the optimizer cache and re-bundle
  --experimentalBundle     Use experimental full bundle mode
  --base <PATH>            Public base path
  -l, --logLevel <LEVEL>   Set log level
  --clearScreen            Allow or disable clearing the screen
  --configLoader <LOADER>  Set the config loader
  -d, --debug [FEAT]       Show debug logs
  -f, --filter <FILTER>    Filter debug logs
  -m, --mode <MODE>        Set env mode
  -h, --help               Print help

Examples:
  vp dev
  vp dev --open
  vp dev --host localhost --port 5173

Documentation: https://viteplus.dev/guide/dev
```

## `vp build --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp build [ROOT] [OPTIONS]

Build for production.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --target <TARGET>             Transpile target
  --outDir <DIR>                Output directory
  --assetsDir <DIR>             Directory for generated assets
  --assetsInlineLimit <NUMBER>  Static asset inline threshold
  --ssr [ENTRY]                 Build for server-side rendering
  --sourcemap [MODE]            Output source maps
  --minify [MINIFIER]           Enable or disable minification
  --manifest [NAME]             Emit a build manifest
  --ssrManifest [NAME]          Emit an SSR manifest
  --emptyOutDir                 Empty outDir even when it is outside root
  -w, --watch                   Rebuild when files change
  --app                         Build an application with the builder API
  --base <PATH>                 Public base path
  -l, --logLevel <LEVEL>        Set log level
  --clearScreen                 Allow or disable clearing the screen
  --configLoader <LOADER>       Set the config loader
  -d, --debug [FEAT]            Show debug logs
  -f, --filter <FILTER>         Filter debug logs
  -m, --mode <MODE>             Set env mode
  -h, --help                    Print help

Examples:
  vp build
  vp build --watch
  vp build --sourcemap

Documentation: https://viteplus.dev/guide/build
```

## `vp preview --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp preview [ROOT] [OPTIONS]

Preview a production build.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --host [HOST]            Specify hostname
  --port <PORT>            Specify port
  --strictPort             Exit if specified port is already in use
  --open [PATH]            Open browser on startup
  --outDir <DIR>           Output directory to preview
  --base <PATH>            Public base path
  -l, --logLevel <LEVEL>   Set log level
  --clearScreen            Allow or disable clearing the screen
  --configLoader <LOADER>  Set the config loader
  -d, --debug [FEAT]       Show debug logs
  -f, --filter <FILTER>    Filter debug logs
  -m, --mode <MODE>        Set env mode
  -h, --help               Print help

Examples:
  vp preview
  vp preview --port 4173

Documentation: https://viteplus.dev/guide/build
```

## `vp test --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp test [COMMAND] [FILTERS] [OPTIONS]

Run tests once by default.
Options are forwarded to Vitest.

Commands:
  run      Run tests once
  watch    Run tests in watch mode
  dev      Run tests in development mode
  related  Run tests related to changed files
  bench    Run benchmarks
  init     Initialize Vitest config
  list     List matching tests

Options:
  -r, --root <PATH>                Set the project root
  -u, --update [TYPE]              Update snapshots
  -w, --watch                      Enable watch mode
  -t, --testNamePattern <PATTERN>  Run tests matching regexp
  --dir <PATH>                     Set the directory to scan for tests
  --ui                             Enable UI
  --open                           Open UI automatically
  --coverage                       Enable coverage
  --reporter <NAME>                Specify reporter
  --browser <NAME>                 Run tests in the browser
  --pool <POOL>                    Set the worker pool
  --maxWorkers <WORKERS>           Set the maximum number of workers
  --environment <NAME>             Set the test environment
  --passWithNoTests                Pass when no tests are found
  --run                            Disable watch mode
  -h, --help                       Print help

Examples:
  vp test
  vp test src/foo.test.ts
  vp test watch --coverage

Documentation: https://viteplus.dev/guide/test
```

## `vp pack --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pack [...FILES] [OPTIONS]

Build a library.
Options are forwarded to Vite+ Pack.

Options:
  --config-loader <LOADER>  Set the config loader
  --no-config               Disable the config file
  -f, --format <FORMAT>     Bundle format: esm, cjs, iife, umd
  -d, --out-dir <DIR>       Output directory
  --target <TARGET>         Bundle target
  --platform <PLATFORM>     Target platform
  --sourcemap               Generate source maps
  --dts                     Generate declaration files
  --minify                  Minify output
  --exe                     Bundle as an executable
  -W, --workspace [DIR]     Enable workspace mode
  -F, --filter <PATTERN>    Filter workspace configs
  -w, --watch [PATH]        Watch mode
  -h, --help                Print help

Examples:
  vp pack
  vp pack src/index.ts --dts
  vp pack --watch

Documentation: https://viteplus.dev/guide/pack
```

## `vp cache --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp cache <COMMAND>

Manage the task cache.

Commands:
  clean  Clean up all the cache

Options:
  -h, --help  Print help

Documentation: https://viteplus.dev/guide/cache
```
