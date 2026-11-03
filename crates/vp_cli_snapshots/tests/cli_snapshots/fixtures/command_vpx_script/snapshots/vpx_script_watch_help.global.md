# vpx_script_watch_help

vpx --watch is the watcher: its help and errors use the vpx --watch name, and --watch-path, which would start Node.js' watch mode, is an error.

## `vpx --watch --help`

```
Usage: vpx --watch [options] [node options] [--] <entry> [script args...]

Run <entry> with oxc-node and run it again when a file it uses changes.

Options:
  --include <glob>         Also watch files matching <glob> (repeatable; "!x" excludes)
  --exclude <glob>         Never watch files matching <glob> (repeatable)
  --clear-screen=<bool>    Clear the screen before each rerun (default: true, TTY only)
  --no-clear-screen        Same as --clear-screen=false
  --kill-signal <signal>   Signal that stops the app before a rerun (default: SIGTERM)
  --kill-timeout <ms>      Force kill the app if it is still running after <ms> (default: 5000)
  --rerun-on-enter=<bool>  Rerun when Return is pressed (default: true on a terminal)
  --no-rerun-on-enter      Never read the terminal; the app keeps stdin to itself
  --poll[=<ms>]            Poll files instead of using file system events (Docker, WSL, network drives)
  --tsconfig <path>        Use this tsconfig (same as OXC_TSCONFIG_PATH)
  -h, --help               Show this help

Node.js options go before <entry> (use --flag=value); everything after <entry> goes to the script.
Globs are relative to the current folder. node_modules is not watched unless --include names it.

Watched: every file the app imports or requires (worker threads on Node 24+), the tsconfig files
oxc-node read (with "extends"), files the resolver looked for and did not find, --env-file files,
Node.js config files (--experimental-config-file).

Keys:  Return        rerun now (press again while stopping to force kill)
       Ctrl+C        stop (press again to force kill)

Examples:
  vpx --watch ./src/server.ts
  vpx --watch --include "./config/*.json" --exclude "src/**/*.test.ts" ./src/server.ts --port 3000
  vpx --watch --inspect --clear-screen=false ./src/index.ts
```

## `vpx --watch`

No entry file

**Exit code:** 1

```
vpx --watch: missing <entry>
Usage: vpx --watch [options] [node options] [--] <entry> [script args...]
Run "vpx --watch --help" for the options.
```

## `vpx --watch-path=src ./src/args.ts`

--watch-path

**Exit code:** 1

```
vpx --watch: --watch-path is not supported by vpx --watch: use --include
Usage: vpx --watch [options] [node options] [--] <entry> [script args...]
Run "vpx --watch --help" for the options.
```
