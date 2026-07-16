# global_cli_fallback

## `vp build -h`

should fall back to global vite-plus and show build help

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

## `vp dev -h`

should fall back to global vite-plus and show dev help

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

## `vp test -h`

should fall back to global vite-plus and show test help

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
