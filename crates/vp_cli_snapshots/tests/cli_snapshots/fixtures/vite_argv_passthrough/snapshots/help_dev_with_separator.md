# help_dev_with_separator

## `vp help dev --`

```
VITE+ - The Unified Toolchain for the Web

Usage: vp dev [ROOT] [OPTIONS]

Run the development server.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --host [host]           [string] specify hostname
  --port <port>           [number] specify port
  --open [path]           [boolean | string] open browser on startup
  --cors                  [boolean] enable CORS
  --strictPort            [boolean] exit if specified port is already in use
  --force                 [boolean] force the optimizer to ignore the cache and re-bundle
  --experimentalBundle    [boolean] use experimental full bundle mode (this is highly experimental)
  --base <path>           [string] public base path (default: /)
  -l, --logLevel <level>  [string] info | warn | error | silent
  --clearScreen           [boolean] allow/disable clear screen when logging
  -d, --debug [feat]      [string | boolean] show debug logs
  -f, --filter <filter>   [string] filter debug logs
  -m, --mode <mode>       [string] set env mode
  --bypass-lock           [boolean] disable process locks
  -h, --help              Display this message

Examples:
  vp dev
  vp dev --open
  vp dev --host localhost --port 5173

Documentation: https://viteplus.dev/guide/dev
```

## `vpt stat-file vite-argv.json --assert missing`

```
vite-argv.json: missing
```
