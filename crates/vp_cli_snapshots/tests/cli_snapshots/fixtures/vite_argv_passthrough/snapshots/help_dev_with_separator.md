# help_dev_with_separator

## `vp help dev --`

```
vp/<version>

Usage:
  $ vp [root]

Commands:
  [root]           start dev server
  build [root]     build for production
  optimize [root]  pre-bundle dependencies (deprecated, the pre-bundle process runs automatically and does not need to be called)
  preview [root]   locally preview production build

For more info, run any command with the `--help` flag:
  $ vp --help
  $ vp build --help
  $ vp optimize --help
  $ vp preview --help

Options:
  --host [host]            [string] specify hostname
  --port <port>            [number] specify port
  --open [path]            [boolean | string] open browser on startup
  --cors                   [boolean] enable CORS
  --strictPort             [boolean] exit if specified port is already in use
  --force                  [boolean] force the optimizer to ignore the cache and re-bundle
  --experimentalBundle     [boolean] use experimental full bundle mode (this is highly experimental)
  -c, --config <file>      [string] use specified config file
  --base <path>            [string] public base path (default: /)
  -l, --logLevel <level>   [string] info | warn | error | silent
  --clearScreen            [boolean] allow/disable clear screen when logging
  --configLoader <loader>  [string] use 'bundle' to bundle the config with Rolldown, or 'runner' (experimental) to process it on the fly, or 'native' (experimental) to load using the native runtime (default: bundle)
  --profile [name]         [boolean | string] start built-in Node.js inspector
  -d, --debug [feat]       [string | boolean] show debug logs
  -f, --filter <filter>    [string] filter debug logs
  -m, --mode <mode>        [string] set env mode
  -h, --help               Display this message
  -v, --version            Display version number
```

## `vpt stat-file vite-argv.json --assert missing`

```
vite-argv.json: missing
```
