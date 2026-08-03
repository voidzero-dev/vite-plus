# command_tool_help_global_delegation

Global and task-script help delegate to the local vite-plus CLI.

## `vp dev --help`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp dev [ROOT] [OPTIONS]

Run the development server.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --host [HOST]           Specify hostname
  --port <PORT>           Specify port
  --open [PATH]           Open browser on startup
  --cors                  Enable CORS
  --strictPort            Exit if specified port is already in use
  --force                 Ignore the optimizer cache and re-bundle
  --experimentalBundle    Use experimental full bundle mode
  --base <PATH>           Public base path
  -l, --logLevel <LEVEL>  Set log level
  --clearScreen           Allow or disable clearing the screen
  -d, --debug [FEAT]      Show debug logs
  -f, --filter <FILTER>   Filter debug logs
  -m, --mode <MODE>       Set env mode
  -h, --help              Print help

Examples:
  vp dev
  vp dev --open
  vp dev --host localhost --port 5173

Documentation: https://viteplus.dev/guide/dev
```

## `vpr localhelp --help`

```
$ vp dev --help --help ⊘ cache disabled
VITE+ - The Unified Toolchain for the Web

Usage: vp dev [ROOT] [OPTIONS]

Run the development server.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --host [HOST]           Specify hostname
  --port <PORT>           Specify port
  --open [PATH]           Open browser on startup
  --cors                  Enable CORS
  --strictPort            Exit if specified port is already in use
  --force                 Ignore the optimizer cache and re-bundle
  --experimentalBundle    Use experimental full bundle mode
  --base <PATH>           Public base path
  -l, --logLevel <LEVEL>  Set log level
  --clearScreen           Allow or disable clearing the screen
  -d, --debug [FEAT]      Show debug logs
  -f, --filter <FILTER>   Filter debug logs
  -m, --mode <MODE>       Set env mode
  -h, --help              Print help

Examples:
  vp dev
  vp dev --open
  vp dev --host localhost --port 5173

Documentation: https://viteplus.dev/guide/dev
```
