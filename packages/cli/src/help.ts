import { renderCliDoc, type CliDoc } from './utils/help.ts';
import { log, printHeader } from './utils/terminal.ts';

// Tool-backed docs mirror upstream even when Vite+ overrides the documented behavior.
const commandHelpDocs = {
  dev: {
    usage: 'vp dev [ROOT] [OPTIONS]',
    summary: ['Run the development server.', 'Options are forwarded to Vite.'],
    sections: [
      {
        title: 'Arguments',
        rows: [
          { label: '[ROOT]', description: 'Project root directory (default: current directory)' },
        ],
      },
      {
        title: 'Options',
        rows: [
          { label: '--host [host]', description: '[string] specify hostname' },
          { label: '--port <port>', description: '[number] specify port' },
          { label: '--open [path]', description: '[boolean | string] open browser on startup' },
          { label: '--cors', description: '[boolean] enable CORS' },
          {
            label: '--strictPort',
            description: '[boolean] exit if specified port is already in use',
          },
          {
            label: '--force',
            description: '[boolean] force the optimizer to ignore the cache and re-bundle',
          },
          {
            label: '--experimentalBundle',
            description:
              '[boolean] use experimental full bundle mode (this is highly experimental)',
          },
          { label: '--base <path>', description: '[string] public base path (default: /)' },
          {
            label: '-l, --logLevel <level>',
            description: '[string] info | warn | error | silent',
          },
          {
            label: '--clearScreen',
            description: '[boolean] allow/disable clear screen when logging',
          },
          { label: '-d, --debug [feat]', description: '[string | boolean] show debug logs' },
          { label: '-f, --filter <filter>', description: '[string] filter debug logs' },
          { label: '-m, --mode <mode>', description: '[string] set env mode' },
          { label: '-h, --help', description: 'Display this message' },
        ],
      },
      {
        title: 'Examples',
        lines: ['  vp dev', '  vp dev --open', '  vp dev --host localhost --port 5173'],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/dev',
  },
  build: {
    usage: 'vp build [ROOT] [OPTIONS]',
    summary: ['Build for production.', 'Options are forwarded to Vite.'],
    sections: [
      {
        title: 'Arguments',
        rows: [
          { label: '[ROOT]', description: 'Project root directory (default: current directory)' },
        ],
      },
      {
        title: 'Options',
        rows: [
          {
            label: '--target <target>',
            description: "[string] transpile target (default: 'baseline-widely-available')",
          },
          { label: '--outDir <dir>', description: '[string] output directory (default: dist)' },
          {
            label: '--assetsDir <dir>',
            description: '[string] directory under outDir to place assets in (default: assets)',
          },
          {
            label: '--assetsInlineLimit <number>',
            description: '[number] static asset base64 inline threshold in bytes (default: 4096)',
          },
          {
            label: '--ssr [entry]',
            description: '[string] build specified entry for server-side rendering',
          },
          {
            label: '--sourcemap [output]',
            description:
              '[boolean | "inline" | "hidden"] output source maps for build (default: false)',
          },
          {
            label: '--minify [minifier]',
            description:
              '[boolean | "oxc" | "terser" | "esbuild"] enable/disable minification, or specify minifier to use (default: oxc)',
          },
          {
            label: '--manifest [name]',
            description: '[boolean | string] emit build manifest json',
          },
          {
            label: '--ssrManifest [name]',
            description: '[boolean | string] emit ssr manifest json',
          },
          {
            label: '--emptyOutDir',
            description: "[boolean] force empty outDir when it's outside of root",
          },
          {
            label: '-w, --watch',
            description: '[boolean] rebuilds when modules have changed on disk',
          },
          { label: '--app', description: '[boolean] same as `builder: {}`' },
          { label: '--base <path>', description: '[string] public base path (default: /)' },
          {
            label: '-l, --logLevel <level>',
            description: '[string] info | warn | error | silent',
          },
          {
            label: '--clearScreen',
            description: '[boolean] allow/disable clear screen when logging',
          },
          { label: '-d, --debug [feat]', description: '[string | boolean] show debug logs' },
          { label: '-f, --filter <filter>', description: '[string] filter debug logs' },
          { label: '-m, --mode <mode>', description: '[string] set env mode' },
          { label: '-h, --help', description: 'Display this message' },
        ],
      },
      {
        title: 'Examples',
        lines: ['  vp build', '  vp build --watch', '  vp build --sourcemap'],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/build',
  },
  preview: {
    usage: 'vp preview [ROOT] [OPTIONS]',
    summary: ['Preview a production build.', 'Options are forwarded to Vite.'],
    sections: [
      {
        title: 'Arguments',
        rows: [
          { label: '[ROOT]', description: 'Project root directory (default: current directory)' },
        ],
      },
      {
        title: 'Options',
        rows: [
          { label: '--host [host]', description: '[string] specify hostname' },
          { label: '--port <port>', description: '[number] specify port' },
          {
            label: '--strictPort',
            description: '[boolean] exit if specified port is already in use',
          },
          { label: '--open [path]', description: '[boolean | string] open browser on startup' },
          { label: '--outDir <dir>', description: '[string] output directory (default: dist)' },
          { label: '--base <path>', description: '[string] public base path (default: /)' },
          {
            label: '-l, --logLevel <level>',
            description: '[string] info | warn | error | silent',
          },
          {
            label: '--clearScreen',
            description: '[boolean] allow/disable clear screen when logging',
          },
          { label: '-d, --debug [feat]', description: '[string | boolean] show debug logs' },
          { label: '-f, --filter <filter>', description: '[string] filter debug logs' },
          { label: '-m, --mode <mode>', description: '[string] set env mode' },
          { label: '-h, --help', description: 'Display this message' },
        ],
      },
      { title: 'Examples', lines: ['  vp preview', '  vp preview --port 4173'] },
    ],
    documentationUrl: 'https://viteplus.dev/guide/build',
  },
  test: {
    usage: 'vp test [COMMAND] [FILTERS]... [OPTIONS]',
    summary: ['Run tests once by default.', 'Options are forwarded to Vitest.'],
    sections: [
      {
        title: 'Commands',
        rows: [
          { label: 'run', description: 'Run tests once' },
          { label: 'watch', description: 'Run tests in watch mode' },
          { label: 'dev', description: 'Run tests in development mode' },
          { label: 'related', description: 'Run tests related to changed files' },
          { label: 'bench', description: 'Run benchmarks' },
          { label: 'list', description: 'List matching tests' },
        ],
      },
      {
        title: 'Arguments',
        rows: [{ label: '[FILTERS]...', description: 'Test file filters' }],
      },
      {
        title: 'Options',
        rows: [
          { label: '-r, --root <path>', description: 'Root path' },
          {
            label: '-u, --update [type]',
            description: 'Update snapshot (accepts boolean, "new", "all" or "none")',
          },
          { label: '-w, --watch', description: 'Enable watch mode' },
          {
            label: '-t, --testNamePattern <pattern>',
            description: 'Run tests with full names matching the specified regexp pattern',
          },
          { label: '--dir <path>', description: 'Base directory to scan for the test files' },
          { label: '--ui', description: 'Enable UI' },
          { label: '--open', description: 'Open UI automatically (default: !process.env.CI)' },
          {
            label: '--api [port]',
            description:
              "Specify server port. Note if the port is already being used, Vite will automatically try the next available port so this may not be the actual port the server ends up listening on. If true will be set to 51204. Use '--help --api' for more info.",
          },
          {
            label: '--silent [value]',
            description:
              "Silent console output from tests. Use 'passed-only' to see logs from failing tests only.",
          },
          { label: '--hideSkippedTests', description: 'Hide logs for skipped tests' },
          {
            label: '--reporter <name>',
            description:
              'Specify reporters (default, agent, minimal, blob, verbose, dot, json, tap, tap-flat, junit, tree, hanging-process, github-actions)',
          },
          {
            label: '--outputFile <filename/-s>',
            description:
              "Write test results to a file when supporter reporter is also specified, use cac's dot notation for individual outputs of multiple reporters (example: --outputFile.tap=./tap.txt)",
          },
          {
            label: '--coverage',
            description: "Enable coverage report. Use '--help --coverage' for more info.",
          },
          {
            label: '--mode <name>',
            description: 'Override Vite mode (default: test or benchmark)',
          },
          {
            label: '--isolate',
            description:
              'Run every test file in isolation. To disable isolation, use --no-isolate (default: true)',
          },
          { label: '--globals', description: 'Inject apis globally' },
          { label: '--dom', description: 'Mock browser API with happy-dom' },
          {
            label: '--browser <name>',
            description:
              "Run tests in the browser. Equivalent to --browser.enabled (default: false). Use '--help --browser' for more info.",
          },
          {
            label: '--pool <pool>',
            description: 'Specify pool, if not running in the browser (default: forks)',
          },
          {
            label: '--execArgv <option>',
            description:
              'Pass additional arguments to node process when spawning worker_threads or child_process.',
          },
          {
            label: '--vmMemoryLimit <limit>',
            description:
              'Memory limit for VM pools. If you see memory leaks, try to tinker this value.',
          },
          {
            label: '--fileParallelism',
            description:
              'Should all test files run in parallel. Use --no-file-parallelism to disable (default: true)',
          },
          {
            label: '--maxWorkers <workers>',
            description: 'Maximum number or percentage of workers to run tests in',
          },
          {
            label: '--environment <name>',
            description:
              'Specify runner environment, if not running in the browser (default: node)',
          },
          { label: '--passWithNoTests', description: 'Pass when no tests are found' },
          {
            label: '--logHeapUsage',
            description: 'Show the size of heap for each test when running in node',
          },
          {
            label: '--detectAsyncLeaks',
            description:
              'Detect asynchronous resources leaking from the test file (default: false)',
          },
          {
            label: '--allowOnly',
            description:
              'Allow tests and suites that are marked as only (default: !process.env.CI)',
          },
          {
            label: '--dangerouslyIgnoreUnhandledErrors',
            description: 'Ignore any unhandled errors that occur',
          },
          {
            label: '--shard <shards>',
            description: 'Test suite shard to execute in a format of <index>/<count>',
          },
          {
            label: '--changed [since]',
            description: 'Run tests that are affected by the changed files (default: false)',
          },
          {
            label: '--sequence <options>',
            description:
              "Options for how tests should be sorted. Use '--help --sequence' for more info.",
          },
          {
            label: '--inspect [[host:]port]',
            description: 'Enable Node.js inspector (default: 127.0.0.1:9229)',
          },
          {
            label: '--inspectBrk [[host:]port]',
            description: 'Enable Node.js inspector and break before the test starts',
          },
          {
            label: '--testTimeout <timeout>',
            description:
              'Default timeout of a test in milliseconds (default: 5000). Use 0 to disable timeout completely.',
          },
          {
            label: '--hookTimeout <timeout>',
            description:
              'Default hook timeout in milliseconds (default: 10000). Use 0 to disable timeout completely.',
          },
          {
            label: '--bail <number>',
            description: 'Stop test execution when given number of tests have failed (default: 0)',
          },
          {
            label: '--retry <times>',
            description:
              "Retry the test specific number of times if it fails (default: 0). Use '--help --retry' for more info.",
          },
          {
            label: '--diff <path>',
            description:
              "DiffOptions object or a path to a module which exports DiffOptions object. Use '--help --diff' for more info.",
          },
          {
            label: '--exclude <glob>',
            description: 'Additional file globs to be excluded from test',
          },
          {
            label: '--expandSnapshotDiff',
            description: 'Show full diff when snapshot fails',
          },
          {
            label: '--disableConsoleIntercept',
            description: 'Disable automatic interception of console logging (default: false)',
          },
          {
            label: '--typecheck',
            description:
              "Enable typechecking alongside tests (default: false). Use '--help --typecheck' for more info.",
          },
          {
            label: '--project <name>',
            description:
              'The name of the project to run if you are using Vitest workspace feature. This can be repeated for multiple projects: --project=1 --project=2. You can also filter projects using wildcards like --project=packages*, and exclude projects with --project=!pattern.',
          },
          {
            label: '--slowTestThreshold <threshold>',
            description:
              'Threshold in milliseconds for a test or suite to be considered slow (default: 300)',
          },
          {
            label: '--teardownTimeout <timeout>',
            description: 'Default timeout of a teardown function in milliseconds (default: 10000)',
          },
          {
            label: '--cache',
            description: "Enable cache. Use '--help --cache' for more info.",
          },
          {
            label: '--maxConcurrency <number>',
            description:
              'Maximum number of concurrent tests and suites during test file execution (default: 5)',
          },
          {
            label: '--expect',
            description:
              "Configuration options for expect() matches. Use '--help --expect' for more info.",
          },
          { label: '--printConsoleTrace', description: 'Always print console stack traces' },
          {
            label: '--includeTaskLocation',
            description: 'Collect test and suite locations in the location property',
          },
          {
            label: '--attachmentsDir <dir>',
            description:
              'The directory where attachments from context.annotate are stored in (default: .vitest-attachments)',
          },
          { label: '--run', description: 'Disable watch mode' },
          {
            label: '--no-color',
            description: 'Removes colors from the console output (default: true)',
          },
          {
            label: '--clearScreen',
            description:
              'Clear terminal screen when re-running tests during watch mode (default: true)',
          },
          {
            label: '--standalone',
            description:
              'Start Vitest without running tests. Tests will be running only on change. If browser mode is enabled, the UI will be opened automatically. This option is ignored when CLI file filters are passed. (default: false)',
          },
          {
            label: '--mergeReports [path]',
            description:
              "Path to a blob reports directory. If this options is used, Vitest won't run any tests, it will only report previously recorded tests",
          },
          {
            label: '--listTags [type]',
            description:
              'List all available tags instead of running tests. --list-tags=json will output tags in JSON format, unless there are no tags.',
          },
          {
            label: '--clearCache',
            description:
              'Delete all Vitest caches, including experimental.fsModuleCache, without running any tests. This will reduce the performance in the subsequent test run.',
          },
          {
            label: '--tagsFilter <expression>',
            description:
              'Run only tests with the specified tags. You can use logical operators && (and), || (or) and ! (not) to create complex expressions, see https://vitest.dev/guide/test-tags#syntax for more information.',
          },
          {
            label: '--strictTags',
            description:
              'Should Vitest throw an error if test has a tag that is not defined in the config. (default: true)',
          },
          {
            label: '--experimental <features>',
            description: "Experimental features.. Use '--help --experimental' for more info.",
          },
          { label: '-h, --help', description: 'Display this message' },
        ],
      },
      {
        title: 'Bench Options',
        rows: [
          {
            label: '--compare <filename>',
            description: 'Benchmark output file to compare against',
          },
          { label: '--outputJson <filename>', description: 'Benchmark output file' },
        ],
      },
      {
        title: 'List Options',
        rows: [
          {
            label: '--json [true/path]',
            description: 'Print collected tests as JSON or write to a file (Default: false)',
          },
          { label: '--filesOnly', description: 'Print only test files with out the test cases' },
          {
            label: '--staticParse',
            description:
              'Parse files statically instead of running them to collect tests (default: false)',
          },
          {
            label: '--staticParseConcurrency <limit>',
            description:
              'How many tests to process at the same time (default: os.availableParallelism())',
          },
        ],
      },
      {
        title: 'Examples',
        lines: ['  vp test', '  vp test src/foo.test.ts', '  vp test watch --coverage'],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/test',
  },
  lint: {
    usage: 'vp lint [PATH]... [OPTIONS]',
    summary: ['Lint code.', 'Options are forwarded to Oxlint.'],
    sections: [
      {
        title: 'Available positional items',
        rows: [
          {
            label: '[PATH]...',
            description: 'Single file, single path or list of paths',
          },
        ],
      },
      {
        title: 'Basic Configuration',
        rows: [
          {
            label: '--tsconfig=<./tsconfig.json>',
            description:
              'Override the TypeScript config used for import resolution. Oxlint automatically discovers the relevant `tsconfig.json` for each file. Use this only when your project uses a non-standard tsconfig name or location.',
          },
        ],
      },
      {
        title: 'Allowing / Denying Multiple Lints',
        lines: [
          '  Accumulate rules and categories from left to right on the command-line.',
          '  For example `-D correctness -A no-debugger` or `-A all -D no-debugger`.',
          '  The categories are:',
          '  * `correctness` - Code that is outright wrong or useless (default)',
          '  * `suspicious`  - Code that is most likely wrong or useless',
          '  * `pedantic`    - Lints which are rather strict or have occasional false positives',
          '  * `perf`        - Code that could be written in a more performant way',
          '  * `style`       - Code that should be written in a more idiomatic way',
          '  * `restriction` - Lints which prevent the use of language and library features',
          '  * `nursery`     - New lints that are still under development',
          '  * `all`         - All categories listed above except `nursery`. Does not enable plugins automatically.',
        ],
        rows: [
          {
            label: '-A, --allow=NAME',
            description: 'Allow the rule or category (suppress the lint)',
          },
          {
            label: '-W, --warn=NAME',
            description: 'Warn on the rule or category (emit a warning)',
          },
          {
            label: '-D, --deny=NAME',
            description: 'Deny the rule or category (emit an error)',
          },
        ],
      },
      {
        title: 'Enable/Disable Plugins',
        rows: [
          {
            label: '--disable-unicorn-plugin',
            description: 'Disable unicorn plugin, which is turned on by default',
          },
          {
            label: '--disable-oxc-plugin',
            description: 'Disable oxc unique rules, which is turned on by default',
          },
          {
            label: '--disable-typescript-plugin',
            description: 'Disable TypeScript plugin, which is turned on by default',
          },
          {
            label: '--import-plugin',
            description: 'Enable import plugin and detect ESM problems.',
          },
          {
            label: '--react-plugin',
            description: 'Enable react plugin, which is turned off by default',
          },
          { label: '--jsdoc-plugin', description: 'Enable jsdoc plugin and detect JSDoc problems' },
          {
            label: '--jest-plugin',
            description: 'Enable the Jest plugin and detect test problems',
          },
          {
            label: '--vitest-plugin',
            description: 'Enable the Vitest plugin and detect test problems',
          },
          {
            label: '--jsx-a11y-plugin',
            description: 'Enable the JSX-a11y plugin and detect accessibility problems',
          },
          {
            label: '--nextjs-plugin',
            description: 'Enable the Next.js plugin and detect Next.js problems',
          },
          {
            label: '--react-perf-plugin',
            description:
              'Enable the React performance plugin and detect rendering performance problems',
          },
          {
            label: '--promise-plugin',
            description: 'Enable the promise plugin and detect promise usage problems',
          },
          {
            label: '--node-plugin',
            description: 'Enable the node plugin and detect node usage problems',
          },
          {
            label: '--vue-plugin',
            description: 'Enable the vue plugin and detect vue usage problems',
          },
        ],
      },
      {
        title: 'Fix Problems',
        rows: [
          {
            label: '--fix',
            description:
              'Fix as many issues as possible. Only unfixed issues are reported in the output.',
          },
          {
            label: '--fix-suggestions',
            description: 'Apply auto-fixable suggestions. May change program behavior.',
          },
          { label: '--fix-dangerously', description: 'Apply dangerous fixes and suggestions' },
        ],
      },
      {
        title: 'Ignore Files',
        rows: [
          {
            label: '--ignore-path=PATH',
            description: 'Specify the file to use as your `.eslintignore`',
          },
          {
            label: '--ignore-pattern=PAT',
            description:
              'Specify patterns of files to ignore (in addition to those in `.eslintignore`)',
          },
          {
            label: '--no-ignore',
            description:
              'Disable excluding files from `.eslintignore` files, --ignore-path flags and --ignore-pattern flags',
          },
        ],
      },
      {
        title: 'Handle Warnings',
        rows: [
          {
            label: '--quiet',
            description: 'Disable reporting on warnings, only errors are reported',
          },
          {
            label: '--deny-warnings',
            description: 'Ensure warnings produce a non-zero exit code',
          },
          {
            label: '--max-warnings=INT',
            description:
              'Specify a warning threshold, which can be used to force exit with an error status if there are too many warning-level rule violations in your project',
          },
        ],
      },
      {
        title: 'Output',
        rows: [
          {
            label: '-f, --format=ARG',
            description:
              'Use a specific output format. Possible values: `checkstyle`, `default`, `agent`, `github`, `gitlab`, `json`, `junit`, `sarif`, `stylish`, `unix`',
          },
          {
            label: '--debug=OPTIONS',
            description: [
              'Enable debug output options. Options are comma-separated. Possible values:',
              ' * `files` - Print the list of files that will be linted, then exit.',
              ' * `timings` - Enable per-rule timing information.',
            ],
          },
        ],
      },
      {
        title: 'Miscellaneous',
        rows: [
          { label: '--silent', description: 'Do not display any diagnostics' },
          {
            label: '--no-error-on-unmatched-pattern',
            description:
              'Do not exit with an error when no files are selected for linting (for example, after applying ignore patterns)',
          },
          {
            label: '--threads=INT',
            description: 'Number of threads to use. Set to 1 for using only 1 CPU core.',
          },
          {
            label: '--print-config',
            description:
              'This option outputs the configuration to be used. When present, no linting is performed and only config-related options are valid.',
          },
        ],
      },
      {
        title: 'Inline Configuration Comments',
        rows: [
          {
            label: '--report-unused-disable-directives',
            description:
              'Report directive comments like `// oxlint-disable-line`, when no errors would have been reported on that line anyway',
          },
          {
            label: '--report-unused-disable-directives-severity=SEVERITY',
            description:
              'Same as `--report-unused-disable-directives`, but allows you to specify the severity level of the reported errors. Only one of these two options can be used at a time.',
          },
        ],
      },
      {
        title: 'Available options',
        rows: [
          { label: '--rules', description: 'List all the rules that are currently registered' },
          { label: '--type-aware', description: 'Enable rules that require type information' },
          {
            label: '--type-check',
            description:
              'Enable experimental type checking (includes TypeScript compiler diagnostics)',
          },
          { label: '-h, --help', description: 'Prints help information' },
        ],
      },
      {
        title: 'Examples',
        lines: [
          '  vp lint',
          '  vp lint src --fix',
          '  vp lint --type-aware --tsconfig ./tsconfig.json',
        ],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/lint',
  },
  fmt: {
    usage: 'vp fmt [PATH]... [OPTIONS]',
    summary: ['Format code.', 'Options are forwarded to Oxfmt.'],
    sections: [
      {
        title: 'Available positional items',
        rows: [
          {
            label: '[PATH]...',
            description:
              "Single file, path or list of paths. Glob patterns are also supported. (Be sure to quote them, otherwise your shell may expand them before passing.) Exclude patterns with `!` prefix like `'!**/fixtures/*.js'` are also supported. If not provided, current working directory is used.",
          },
        ],
      },
      {
        title: 'Mode Options',
        rows: [
          {
            label: '--stdin-filepath=PATH',
            description: 'Specify the file name to use to infer which parser to use',
          },
        ],
      },
      {
        title: 'Output Options',
        rows: [
          { label: '--write', description: 'Format and write files in place (default)' },
          {
            label: '--check',
            description: 'Check if files are formatted, also show statistics',
          },
          { label: '--list-different', description: 'List files that would be changed' },
        ],
      },
      {
        title: 'Ignore Options',
        rows: [
          {
            label: '--ignore-path=PATH',
            description:
              'Path to ignore file(s). Can be specified multiple times. If not specified, .gitignore and .prettierignore in the current directory are used.',
          },
          {
            label: '--with-node-modules',
            description: 'Format code in node_modules directory (skipped by default)',
          },
        ],
      },
      {
        title: 'Runtime Options',
        rows: [
          {
            label: '--no-error-on-unmatched-pattern',
            description: 'Do not exit with error when pattern is unmatched',
          },
          {
            label: '--threads=INT',
            description: 'Number of threads to use. Set to 1 for using only 1 CPU core.',
          },
        ],
      },
      {
        title: 'Available options',
        rows: [{ label: '-h, --help', description: 'Prints help information' }],
      },
      { title: 'Examples', lines: ['  vp fmt', '  vp fmt src --check', '  vp fmt . --write'] },
    ],
    documentationUrl: 'https://viteplus.dev/guide/fmt',
  },
  check: {
    usage: 'vp check [OPTIONS] [PATHS]...',
    summary: 'Run format, lint, and type checks.',
    sections: [
      {
        title: 'Arguments',
        rows: [{ label: '[PATHS]...', description: 'File paths to pass to fmt and lint' }],
      },
      {
        title: 'Options',
        rows: [
          { label: '--fix', description: 'Auto-fix format and lint issues' },
          {
            label: '--quiet',
            description: 'Disable reporting on warnings, only errors are reported',
          },
          { label: '--no-fmt', description: 'Skip format check' },
          {
            label: '--no-lint',
            description:
              'Skip lint rules; type-check still runs when `lint.options.typeCheck` is true',
          },
          {
            label: '--no-error-on-unmatched-pattern',
            description: 'Do not exit with error when pattern is unmatched',
          },
          { label: '-h, --help', description: 'Print help' },
        ],
      },
      {
        title: 'Examples',
        lines: ['  vp check', '  vp check --fix', '  vp check --no-lint src/index.ts'],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/check',
  },
  pack: {
    usage: 'vp pack [...files] [OPTIONS]',
    summary: ['Build a library.', 'Options are forwarded to Vite+ Pack.'],
    sections: [
      {
        title: 'Arguments',
        rows: [{ label: '[...files]', description: 'Bundle files' }],
      },
      {
        title: 'Options',
        rows: [
          { label: '--no-config', description: 'Disable config file' },
          {
            label: '-f, --format <format>',
            description: 'Bundle format: esm, cjs, iife, umd (default: esm)',
          },
          { label: '--clean', description: 'Clean output directory, --no-clean to disable' },
          {
            label: '--deps.never-bundle <module>',
            description: 'Mark dependencies as external',
          },
          { label: '--minify', description: 'Minify output' },
          { label: '--devtools', description: 'Enable devtools integration' },
          { label: '--debug [feat]', description: 'Show debug logs' },
          {
            label: '--target <target>',
            description: 'Bundle target, e.g "es2015", "esnext"',
          },
          {
            label: '-l, --logLevel <level>',
            description: 'Set log level: info, warn, error, silent',
          },
          { label: '--fail-on-warn', description: 'Fail on warnings (default: true)' },
          {
            label: '--no-write',
            description:
              'Disable writing files to disk, incompatible with watch mode (default: true)',
          },
          { label: '-d, --out-dir <dir>', description: 'Output directory (default: dist)' },
          { label: '--treeshake', description: 'Tree-shake bundle (default: true)' },
          { label: '--sourcemap', description: 'Generate source map (default: false)' },
          { label: '--shims', description: 'Enable cjs and esm shims (default: false)' },
          { label: '--platform <platform>', description: 'Target platform (default: node)' },
          { label: '--dts', description: 'Generate dts files' },
          { label: '--publint', description: 'Enable publint (default: false)' },
          {
            label: '--attw',
            description: 'Enable Are the types wrong integration (default: false)',
          },
          {
            label: '--unused',
            description: 'Enable unused dependencies check (default: false)',
          },
          { label: '-w, --watch [path]', description: 'Watch mode' },
          { label: '--ignore-watch <path>', description: 'Ignore custom paths in watch mode' },
          { label: '--from-vite [vitest]', description: 'Reuse config from Vite or Vitest' },
          { label: '--report', description: 'Size report (default: true)' },
          {
            label: '--env.* <value>',
            description: 'Define compile-time env variables',
          },
          {
            label: '--env-file <file>',
            description:
              'Load environment variables from a file, when used together with --env, variables in --env take precedence',
          },
          {
            label: '--env-prefix <prefix>',
            description: 'Prefix for env variables to inject into the bundle (default: TSDOWN_)',
          },
          { label: '--on-success <command>', description: 'Command to run on success' },
          { label: '--copy <dir>', description: 'Copy files to output dir' },
          { label: '--public-dir <dir>', description: 'Alias for --copy, deprecated' },
          { label: '--tsconfig <tsconfig>', description: 'Set tsconfig path' },
          { label: '--unbundle', description: 'Unbundle mode' },
          { label: '--root <dir>', description: 'Root directory of input files' },
          { label: '--exe', description: 'Bundle as executable' },
          { label: '-W, --workspace [dir]', description: 'Enable workspace mode' },
          {
            label: '--concurrency <count>',
            description: 'Maximum number of Rolldown builds to run in parallel',
          },
          {
            label: '-F, --filter <pattern>',
            description: 'Filter configs (cwd or name), e.g. /pkg-name$/ or pkg-name',
          },
          {
            label: '--exports',
            description: 'Generate export-related metadata for package.json (experimental)',
          },
          { label: '-h, --help', description: 'Display this message' },
        ],
      },
      {
        title: 'Examples',
        lines: ['  vp pack', '  vp pack src/index.ts --dts', '  vp pack --watch'],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/pack',
  },
  run: {
    usage: 'vp run [OPTIONS] [TASK_SPECIFIER] [ADDITIONAL_ARGS]...',
    summary: 'Run tasks.',
    sections: [
      {
        title: 'Arguments',
        rows: [
          {
            label: '[TASK_SPECIFIER]',
            description:
              '`packageName#taskName` or `taskName`. If omitted, shows the task selector',
          },
          {
            label: '[ADDITIONAL_ARGS]...',
            description: 'Additional arguments to pass to the task',
          },
        ],
      },
      {
        title: 'Options',
        rows: [
          { label: '-r, --recursive', description: 'Select all packages in the workspace' },
          {
            label: '-t, --transitive',
            description: 'Select the current package and its transitive dependencies',
          },
          { label: '-w, --workspace-root', description: 'Select the workspace root package' },
          {
            label: '-F, --filter <FILTERS>',
            description: 'Match packages by name, directory, or glob pattern',
          },
          {
            label: '--fail-if-no-match',
            description: 'Exit with a non-zero status if a filter matches no packages',
          },
          {
            label: '--ignore-depends-on',
            description: 'Do not run dependencies specified in `dependsOn` fields',
          },
          { label: '-v, --verbose', description: 'Show the full detailed summary after execution' },
          { label: '--cache', description: 'Force caching on for all tasks and scripts' },
          { label: '--no-cache', description: 'Force caching off for all tasks and scripts' },
          {
            label: '--log <MODE>',
            description: 'Set output mode: interleaved (default), labeled, or grouped',
          },
          {
            label: '--concurrency-limit <N>',
            description: 'Maximum number of tasks to run concurrently (default: 4)',
          },
          {
            label: '--parallel',
            description:
              'Run tasks without dependency ordering; concurrency is unlimited unless `--concurrency-limit` is specified',
          },
          { label: '--last-details', description: 'Display the detailed summary of the last run' },
          { label: '-h, --help', description: 'Print help' },
        ],
      },
      {
        title: 'Filter Patterns',
        lines: [
          '  --filter <pattern>        Select by package name (e.g. foo, @scope/*)',
          '  --filter ./<dir>          Select packages under a directory',
          '  --filter {<dir>}          Same as ./<dir>, but allows traversal suffixes',
          '  --filter <pattern>...     Select package and its dependencies',
          '  --filter ...<pattern>     Select package and its dependents',
          '  --filter <pattern>^...    Select only the dependencies (exclude the package itself)',
          '  --filter !<pattern>       Exclude packages matching the pattern',
        ],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/run',
  },
  exec: {
    usage: 'vp exec [OPTIONS] [COMMAND]...',
    summary: 'Execute a command from local node_modules/.bin.',
    sections: [
      {
        title: 'Arguments',
        rows: [{ label: '[COMMAND]...', description: 'Command and arguments to execute' }],
      },
      {
        title: 'Options',
        rows: [
          { label: '-r, --recursive', description: 'Select all packages in the workspace' },
          {
            label: '-t, --transitive',
            description: 'Select the current package and its transitive dependencies',
          },
          { label: '-w, --workspace-root', description: 'Select the workspace root package' },
          {
            label: '-F, --filter <FILTERS>',
            description: 'Match packages by name, directory, or glob pattern',
          },
          {
            label: '--fail-if-no-match',
            description: [
              'Exit with a non-zero status if a `--filter` expression matches no packages',
              'Without this flag, unmatched filters only warn and exit successfully',
            ],
          },
          {
            label: '-c, --shell-mode',
            description: 'Execute the command within a shell environment',
          },
          { label: '--parallel', description: 'Run concurrently without topological ordering' },
          { label: '--reverse', description: 'Reverse execution order' },
          { label: '--resume-from <PACKAGE>', description: 'Resume from a specific package' },
          { label: '--report-summary', description: 'Save results to vp-exec-summary.json' },
          { label: '-h, --help', description: 'Print help' },
        ],
      },
      {
        title: 'Filter Patterns',
        lines: [
          '  --filter <pattern>        Select by package name (e.g. foo, @scope/*)',
          '  --filter ./<dir>          Select packages under a directory',
          '  --filter {<dir>}          Same as ./<dir>, but allows traversal suffixes',
          '  --filter <pattern>...     Select package and its dependencies',
          '  --filter ...<pattern>     Select package and its dependents',
          '  --filter <pattern>^...    Select only the dependencies (exclude the package itself)',
          '  --filter !<pattern>       Exclude packages matching the pattern',
        ],
      },
      {
        title: 'Examples',
        lines: [
          '  vp exec node --version                             # Run local node',
          '  vp exec tsc --noEmit                               # Run local TypeScript compiler',
          "  vp exec -c 'tsc --noEmit && prettier --check .'    # Shell mode",
          '  vp exec -r -- tsc --noEmit                         # Run in all workspace packages',
          "  vp exec --filter 'app...' -- tsc                   # Run in filtered packages",
        ],
      },
    ],
    documentationUrl: 'https://viteplus.dev/guide/vpx',
  },
  cache: {
    usage: 'vp cache <COMMAND>',
    summary: 'Manage the task cache.',
    sections: [
      { title: 'Commands', rows: [{ label: 'clean', description: 'Clean up all the cache' }] },
      { title: 'Options', rows: [{ label: '-h, --help', description: 'Print help' }] },
    ],
    documentationUrl: 'https://viteplus.dev/guide/cache',
  },
} satisfies Record<string, CliDoc>;

export function maybePrintCommandHelp(args: readonly string[]): boolean {
  const command = args[0] === 'format' ? 'fmt' : args[0];
  const doc = commandHelpDocs[command as keyof typeof commandHelpDocs];
  if (!doc) {
    return false;
  }

  const commandArgs = args.slice(1);
  const isTopLevelHelp =
    commandArgs.length === 1 && (commandArgs[0] === '-h' || commandArgs[0] === '--help');
  if (!isTopLevelHelp) {
    return false;
  }

  printHeader();
  log(renderCliDoc(doc));
  return true;
}
