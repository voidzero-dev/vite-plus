# command_vpx_pnpm11

## `vpx --help`

should show vpx help message

```
Execute a command from a local or remote npm package, or run a script file

Usage: vpx [OPTIONS] <pkg[@version]> [args...]
       vpx [OPTIONS] [NODE_OPTIONS] <script> [args...]

Arguments:
  <pkg[@version]>  Package binary to execute
  <script>         Script file to run (.ts, .mts, .cts, .tsx, .js, .mjs, .cjs, .jsx)
  [args...]        Arguments to pass to the command or script

Options:
  -p, --package <NAME>  Package(s) to install if not found locally
  -c, --shell-mode      Execute the command within a shell environment
  -s, --silent          Suppress all output except the command's output
      --tsconfig <PATH> tsconfig.json to use when running a script
  -v, --version         Print the Vite+ version
  -h, --help            Print help

Before a script, --watch reruns it when a file it uses changes (see `vpx --watch --help`).

Examples:
  vpx eslint .                           # Run local eslint (or download)
  vpx create-vue my-app                  # Download and run create-vue
  vpx oxlint@1.85.0 --version            # Run specific version
  vpx -p cowsay -c 'echo "hi" | cowsay'  # Shell mode with package
  vpx ./scripts/seed.ts --dry-run        # Run a TypeScript script
  vpx --inspect ./server.ts              # Pass Node.js options before the script
  vpx --watch ./server.ts                # Rerun the script when a file it uses changes
```

## `vpx -s vp-test-echo hello`

should run vp-test-echo via dlx fallback

```
hello
```
