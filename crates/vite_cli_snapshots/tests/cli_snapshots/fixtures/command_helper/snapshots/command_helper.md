# command_helper

## `vp -h`

help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp <COMMAND>

Core Commands:
  create         Create a new project from a template
  migrate        Migrate an existing project to Vite+
  dev            Run the development server
  build          Build for production
  test           Run tests
  lint           Lint code
  fmt, format    Format code
  check          Run format, lint, and type checks
  pack           Build library
  run            Run tasks
  exec           Execute a command from local node_modules/.bin
  preview        Preview production build
  cache          Manage the task cache
  config         Configure hooks and agent integration
  staged         Run linters on staged files

Package Manager Commands:
  install    Install all dependencies, or add packages if package names are provided

Options:
  -h, --help  Print help
```

## `vp pack -h`

pack help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp pack [...FILES] [OPTIONS]

Build a library.
Options are forwarded to Vite+ Pack.

Arguments:
  [...FILES]  Files to bundle

Options:
  -f, --format <FORMAT>         Bundle format: esm, cjs, iife, umd (default: esm)
  --clean                       Clean output directory, --no-clean to disable
  --deps.never-bundle <MODULE>  Mark dependencies as external
  --minify                      Minify output
  --devtools                    Enable devtools integration
  --debug [FEAT]                Show debug logs
  --target <TARGET>             Bundle target, e.g "es2015", "esnext"
  -l, --logLevel <LEVEL>        Set log level: info, warn, error, silent
  --fail-on-warn                Fail on warnings (default: true)
  --no-write                    Disable writing files to disk, incompatible with watch mode (default: true)
  -d, --out-dir <DIR>           Output directory (default: dist)
  --treeshake                   Tree-shake bundle (default: true)
  --sourcemap                   Generate source map (default: false)
  --shims                       Enable cjs and esm shims (default: false)
  --platform <PLATFORM>         Target platform (default: node)
  --dts                         Generate dts files
  --publint                     Enable publint (default: false)
  --attw                        Enable Are the types wrong integration (default: false)
  --unused                      Enable unused dependencies check (default: false)
  -w, --watch [PATH]            Watch mode
  --ignore-watch <PATH>         Ignore custom paths in watch mode
  --from-vite [VITEST]          Reuse config from Vite or Vitest
  --report                      Size report (default: true)
  --env.* <VALUE>               Define compile-time env variables
  --env-file <FILE>             Load environment variables from a file, when used together with --env, variables in --env take precedence
  --env-prefix <PREFIX>         Prefix for env variables to inject into the bundle (default: VITE_PACK_,TSDOWN_)
  --on-success <COMMAND>        Command to run on success
  --copy <DIR>                  Copy files to output dir
  --public-dir <DIR>            Alias for --copy, deprecated
  --tsconfig <TSCONFIG>         Set tsconfig path
  --unbundle                    Unbundle mode
  --root <DIR>                  Root directory of input files
  --exe                         Bundle as executable
  -W, --workspace [DIR]         Enable workspace mode
  -F, --filter <PATTERN>        Filter configs (cwd or name), e.g. /pkg-name$/ or pkg-name
  --exports                     Generate export-related metadata for package.json (experimental)
  -h, --help                    Display this message

Examples:
  vp pack
  vp pack src/index.ts --dts
  vp pack --watch

Documentation: https://viteplus.dev/guide/pack
```

## `vp fmt -h`

fmt help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp fmt [PATH]... [OPTIONS]

Format code.
Options are forwarded to Oxfmt.

Arguments:
  [PATH]...  Files, directories, or glob patterns (default: current directory)

Mode Options:
  --stdin-filepath <PATH>  Specify the file name used to infer the parser for stdin

Output Options:
  --write           Format and write files in place
  --check           Check whether files are formatted and show statistics
  --list-different  List files that would be changed

Ignore Options:
  --ignore-path <PATH>  Path to an ignore file; may be specified multiple times
  --with-node-modules   Format files in node_modules, which are skipped by default

Runtime Options:
  --no-error-on-unmatched-pattern  Do not exit with an error when the pattern is unmatched
  --threads <INT>                  Number of threads to use; set to 1 to use one CPU core

Options:
  -h, --help  Print help information

Examples:
  vp fmt
  vp fmt src --check
  vp fmt . --write

Documentation: https://viteplus.dev/guide/fmt
```

## `vp lint -h`

lint help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp lint [PATH]... [OPTIONS]

Lint code.
Options are forwarded to Oxlint.

Arguments:
  [PATH]...  Files or directories to lint

Basic Configuration:
  --tsconfig <PATH>  Override the TypeScript config used for import resolution

Rule Severity:
  -A, --allow <NAME>  Allow a rule or category
  -W, --warn <NAME>   Emit a warning for a rule or category
  -D, --deny <NAME>   Emit an error for a rule or category

Plugins:
  --disable-unicorn-plugin     Disable the unicorn plugin, which is enabled by default
  --disable-oxc-plugin         Disable Oxc-specific rules, which are enabled by default
  --disable-typescript-plugin  Disable the TypeScript plugin, which is enabled by default
  --import-plugin              Enable the import plugin
  --react-plugin               Enable the React plugin
  --jsdoc-plugin               Enable the JSDoc plugin
  --jest-plugin                Enable the Jest plugin
  --vitest-plugin              Enable the Vitest plugin
  --jsx-a11y-plugin            Enable the JSX accessibility plugin
  --nextjs-plugin              Enable the Next.js plugin
  --react-perf-plugin          Enable the React performance plugin
  --promise-plugin             Enable the promise plugin
  --node-plugin                Enable the Node.js plugin
  --vue-plugin                 Enable the Vue plugin

Fix Problems:
  --fix              Fix issues when possible
  --fix-suggestions  Apply auto-fixable suggestions
  --fix-dangerously  Apply dangerous fixes and suggestions

Ignore Files:
  --ignore-path <PATH>        Use the specified .eslintignore file
  --ignore-pattern <PATTERN>  Add file patterns to ignore
  --no-ignore                 Disable file exclusion from ignore rules

Handle Warnings:
  --quiet               Report errors only
  --deny-warnings       Exit non-zero when warnings are reported
  --max-warnings <INT>  Set the warning threshold before exiting non-zero

Output:
  -f, --format <FORMAT>  Set output format: checkstyle, default, agent, github, gitlab, json, junit, sarif, stylish, or unix
  --debug <OPTIONS>      Enable comma-separated debug output options: files or timings

Miscellaneous:
  --silent                         Do not display diagnostics
  --no-error-on-unmatched-pattern  Do not exit with an error when no files are selected for linting
  --threads <INT>                  Number of threads to use; set to 1 to use one CPU core
  --print-config                   Print the resolved configuration without linting

Inline Configuration:
  --report-unused-disable-directives                      Report unused oxlint-disable directives
  --report-unused-disable-directives-severity <SEVERITY>  Report unused disable directives at the specified severity

Options:
  --rules       List all registered rules
  --type-aware  Enable rules requiring type information
  --type-check  Enable experimental type checking and compiler diagnostics
  -h, --help    Print help information

Examples:
  vp lint
  vp lint src --fix
  vp lint --type-aware --tsconfig ./tsconfig.json

Documentation: https://viteplus.dev/guide/lint
```

## `vp build -h`

build help message

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

## `vp test -h`

test help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp test [COMMAND] [FILTERS]... [OPTIONS]

Run tests once by default.
Options are forwarded to Vitest.

Commands:
  run      Run tests once
  watch    Run tests in watch mode
  dev      Run tests in development mode
  related  Run tests related to changed files
  bench    Run benchmarks
  list     List matching tests

Arguments:
  [FILTERS]...  Test file filters

Options:
  -r, --root <PATH>                   Root path
  -u, --update [TYPE]                 Update snapshot (accepts boolean, "new", "all" or "none")
  -w, --watch                         Enable watch mode
  -t, --testNamePattern <PATTERN>     Run tests with full names matching the specified regexp pattern
  --dir <PATH>                        Base directory to scan for the test files
  --ui                                Enable UI
  --open                              Open UI automatically (default: !process.env.CI)
  --api [PORT]                        Specify server port; if true, defaults to 51204
  --silent [VALUE]                    Silent console output from tests. Use 'passed-only' to see logs from failing tests only
  --hideSkippedTests                  Hide logs for skipped tests
  --reporter <NAME>                   Specify reporters (default, agent, minimal, blob, verbose, dot, json, tap, tap-flat, junit, tree, hanging-process, github-actions)
  --outputFile <FILENAME/-S>          Write test results to a file; use dot notation for individual outputs of multiple reporters (for example, --outputFile.tap=./tap.txt)
  --coverage                          Enable coverage reporting
  --mode <NAME>                       Override Vite mode (default: test or benchmark)
  --isolate                           Run every test file in isolation. Use --no-isolate to disable (default: true)
  --globals                           Inject APIs globally
  --dom                               Mock browser API with happy-dom
  --browser <NAME>                    Run tests in the browser; equivalent to --browser.enabled (default: false)
  --pool <POOL>                       Specify pool when not running in the browser (default: forks)
  --execArgv <OPTION>                 Pass additional arguments to Node.js when spawning worker threads or child processes
  --vmMemoryLimit <LIMIT>             Memory limit for VM pools
  --fileParallelism                   Run test files in parallel. Use --no-file-parallelism to disable (default: true)
  --maxWorkers <WORKERS>              Maximum number or percentage of workers to run tests in
  --environment <NAME>                Specify runner environment (default: node)
  --passWithNoTests                   Pass when no tests are found
  --logHeapUsage                      Show the size of the heap for each test when running in Node.js
  --detectAsyncLeaks                  Detect asynchronous resources leaking from test files (default: false)
  --allowOnly                         Allow tests and suites marked as only (default: !process.env.CI)
  --dangerouslyIgnoreUnhandledErrors  Ignore any unhandled errors that occur
  --shard <SHARDS>                    Test suite shard to execute in the format <index>/<count>
  --changed [SINCE]                   Run tests affected by changed files (default: false)
  --sequence <OPTIONS>                Configure test sorting
  --inspect [[HOST:]PORT]             Enable Node.js inspector (default: 127.0.0.1:9229)
  --inspectBrk [[HOST:]PORT]          Enable Node.js inspector and break before tests start
  --testTimeout <TIMEOUT>             Default test timeout in milliseconds (default: 5000; 0 disables)
  --hookTimeout <TIMEOUT>             Default hook timeout in milliseconds (default: 10000; 0 disables)
  --bail <NUMBER>                     Stop test execution after the given number of failures (default: 0)
  --retry <TIMES>                     Retry failed tests (default: 0)
  --diff <PATH>                       DiffOptions object or path to a module exporting one
  --exclude <GLOB>                    Additional file globs to exclude from tests
  --expandSnapshotDiff                Show the full diff when a snapshot fails
  --disableConsoleIntercept           Disable automatic interception of console logging (default: false)
  --typecheck                         Enable typechecking alongside tests (default: false)
  --project <NAME>                    Select one or more Vitest workspace projects by name or wildcard
  --slowTestThreshold <THRESHOLD>     Threshold for a test or suite to be considered slow (default: <duration>)
  --teardownTimeout <TIMEOUT>         Default teardown timeout in milliseconds (default: 10000)
  --cache                             Enable cache
  --maxConcurrency <NUMBER>           Maximum number of concurrent tests and suites (default: 5)
  --expect                            Configure expect matchers
  --printConsoleTrace                 Always print console stack traces
  --includeTaskLocation               Collect test and suite locations in the location property
  --attachmentsDir <DIR>              Directory for attachments created with context.annotate (default: .vitest-attachments)
  --run                               Disable watch mode
  --no-color                          Remove colors from console output (default: true)
  --clearScreen                       Clear the terminal when rerunning tests in watch mode (default: true)
  --standalone                        Start Vitest without running tests until files change (default: false)
  --mergeReports [PATH]               Merge previously recorded blob reports without running tests
  --listTags [TYPE]                   List available tags; --list-tags=json outputs JSON
  --clearCache                        Delete all Vitest caches without running tests
  --tagsFilter <EXPRESSION>           Run only tests matching the tag expression
  --strictTags                        Error when a test uses an undefined tag (default: true)
  --experimental <FEATURES>           Enable experimental features
  -h, --help                          Display this message

Bench Options:
  --compare <FILENAME>     Benchmark output file to compare against
  --outputJson <FILENAME>  Benchmark output file

List Options:
  --json [TRUE/PATH]                Print collected tests as JSON or write to a file (default: false)
  --filesOnly                       Print only test files without test cases
  --staticParse                     Parse files statically instead of running them (default: false)
  --staticParseConcurrency <LIMIT>  Number of test files to process concurrently

Examples:
  vp test
  vp test src/foo.test.ts
  vp test watch --coverage

Documentation: https://viteplus.dev/guide/test
```

## `vp preview -h`

preview help message

```
VITE+ - The Unified Toolchain for the Web

Usage: vp preview [ROOT] [OPTIONS]

Preview a production build.
Options are forwarded to Vite.

Arguments:
  [ROOT]  Project root directory (default: current directory)

Options:
  --host [HOST]           Specify hostname
  --port <PORT>           Specify port
  --strictPort            Exit if specified port is already in use
  --open [PATH]           Open browser on startup
  --outDir <DIR>          Output directory to preview
  --base <PATH>           Public base path
  -l, --logLevel <LEVEL>  Set log level
  --clearScreen           Allow or disable clearing the screen
  -d, --debug [FEAT]      Show debug logs
  -f, --filter <FILTER>   Filter debug logs
  -m, --mode <MODE>       Set env mode
  -h, --help              Print help

Examples:
  vp preview
  vp preview --port 4173

Documentation: https://viteplus.dev/guide/build
```

## `vp dev -h`

dev help message

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
