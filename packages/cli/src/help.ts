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
          { label: '--host [HOST]', description: 'Specify hostname' },
          { label: '--port <PORT>', description: 'Specify port' },
          { label: '--open [PATH]', description: 'Open browser on startup' },
          { label: '--cors', description: 'Enable CORS' },
          { label: '--strictPort', description: 'Exit if specified port is already in use' },
          { label: '--force', description: 'Ignore the optimizer cache and re-bundle' },
          { label: '--experimentalBundle', description: 'Use experimental full bundle mode' },
          { label: '-c, --config <FILE>', description: 'Use the specified config file' },
          { label: '--base <PATH>', description: 'Public base path' },
          { label: '-l, --logLevel <LEVEL>', description: 'Set log level' },
          { label: '--clearScreen', description: 'Allow or disable clearing the screen' },
          {
            label: '--configLoader <LOADER>',
            description: 'Set the config loader: bundle, runner, or native (default: bundle)',
          },
          { label: '-d, --debug [FEAT]', description: 'Show debug logs' },
          { label: '-f, --filter <FILTER>', description: 'Filter debug logs' },
          { label: '-m, --mode <MODE>', description: 'Set env mode' },
          { label: '-h, --help', description: 'Print help' },
          { label: '-v, --version', description: 'Display version number' },
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
          { label: '--target <TARGET>', description: 'Transpile target' },
          { label: '--outDir <DIR>', description: 'Output directory' },
          { label: '--assetsDir <DIR>', description: 'Directory for generated assets' },
          { label: '--assetsInlineLimit <NUMBER>', description: 'Static asset inline threshold' },
          { label: '--ssr [ENTRY]', description: 'Build for server-side rendering' },
          { label: '--sourcemap [MODE]', description: 'Output source maps' },
          { label: '--minify [MINIFIER]', description: 'Enable or disable minification' },
          { label: '--manifest [NAME]', description: 'Emit a build manifest' },
          { label: '--ssrManifest [NAME]', description: 'Emit an SSR manifest' },
          { label: '--emptyOutDir', description: 'Empty outDir even when it is outside root' },
          { label: '-w, --watch', description: 'Rebuild when files change' },
          { label: '--app', description: 'Build an application with the builder API' },
          { label: '-c, --config <FILE>', description: 'Use the specified config file' },
          { label: '--base <PATH>', description: 'Public base path' },
          { label: '-l, --logLevel <LEVEL>', description: 'Set log level' },
          { label: '--clearScreen', description: 'Allow or disable clearing the screen' },
          {
            label: '--configLoader <LOADER>',
            description: 'Set the config loader: bundle, runner, or native (default: bundle)',
          },
          { label: '-d, --debug [FEAT]', description: 'Show debug logs' },
          { label: '-f, --filter <FILTER>', description: 'Filter debug logs' },
          { label: '-m, --mode <MODE>', description: 'Set env mode' },
          { label: '-h, --help', description: 'Print help' },
          { label: '-v, --version', description: 'Display version number' },
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
          { label: '--host [HOST]', description: 'Specify hostname' },
          { label: '--port <PORT>', description: 'Specify port' },
          { label: '--strictPort', description: 'Exit if specified port is already in use' },
          { label: '--open [PATH]', description: 'Open browser on startup' },
          { label: '--outDir <DIR>', description: 'Output directory to preview' },
          { label: '-c, --config <FILE>', description: 'Use the specified config file' },
          { label: '--base <PATH>', description: 'Public base path' },
          { label: '-l, --logLevel <LEVEL>', description: 'Set log level' },
          { label: '--clearScreen', description: 'Allow or disable clearing the screen' },
          {
            label: '--configLoader <LOADER>',
            description: 'Set the config loader: bundle, runner, or native (default: bundle)',
          },
          { label: '-d, --debug [FEAT]', description: 'Show debug logs' },
          { label: '-f, --filter <FILTER>', description: 'Filter debug logs' },
          { label: '-m, --mode <MODE>', description: 'Set env mode' },
          { label: '-h, --help', description: 'Print help' },
          { label: '-v, --version', description: 'Display version number' },
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
          { label: 'init <PROJECT>', description: 'Initialize a Vitest project' },
          { label: 'list', description: 'List matching tests' },
          { label: 'complete [SHELL]', description: 'Set up shell completion' },
        ],
      },
      {
        title: 'Arguments',
        rows: [{ label: '[FILTERS]...', description: 'Test file filters' }],
      },
      {
        title: 'Options',
        rows: [
          { label: '-v, --version', description: 'Display version number' },
          { label: '-r, --root <PATH>', description: 'Root path' },
          { label: '-c, --config <PATH>', description: 'Path to config file' },
          {
            label: '-u, --update [TYPE]',
            description: 'Update snapshot (accepts boolean, "new", "all" or "none")',
          },
          { label: '-w, --watch', description: 'Enable watch mode' },
          {
            label: '-t, --testNamePattern <PATTERN>',
            description: 'Run tests with full names matching the specified regexp pattern',
          },
          { label: '--dir <PATH>', description: 'Base directory to scan for the test files' },
          { label: '--ui', description: 'Enable UI' },
          { label: '--open', description: 'Open UI automatically (default: !process.env.CI)' },
          {
            label: '--api [PORT]',
            description: 'Specify server port; if true, defaults to 51204',
          },
          {
            label: '--silent [VALUE]',
            description:
              "Silent console output from tests. Use 'passed-only' to see logs from failing tests only",
          },
          { label: '--hideSkippedTests', description: 'Hide logs for skipped tests' },
          {
            label: '--reporter <NAME>',
            description:
              'Specify reporters (default, agent, minimal, blob, verbose, dot, json, tap, tap-flat, junit, tree, hanging-process, github-actions)',
          },
          {
            label: '--outputFile <FILENAME/-S>',
            description:
              'Write test results to a file; use dot notation for individual outputs of multiple reporters (for example, --outputFile.tap=./tap.txt)',
          },
          {
            label: '--coverage',
            description: 'Enable coverage reporting',
          },
          {
            label: '--mode <NAME>',
            description: 'Override Vite mode (default: test or benchmark)',
          },
          {
            label: '--isolate',
            description:
              'Run every test file in isolation. Use --no-isolate to disable (default: true)',
          },
          { label: '--globals', description: 'Inject APIs globally' },
          { label: '--dom', description: 'Mock browser API with happy-dom' },
          {
            label: '--browser <NAME>',
            description:
              'Run tests in the browser; equivalent to --browser.enabled (default: false)',
          },
          {
            label: '--pool <POOL>',
            description: 'Specify pool when not running in the browser (default: forks)',
          },
          {
            label: '--execArgv <OPTION>',
            description:
              'Pass additional arguments to Node.js when spawning worker threads or child processes',
          },
          {
            label: '--vmMemoryLimit <LIMIT>',
            description: 'Memory limit for VM pools',
          },
          {
            label: '--fileParallelism',
            description:
              'Run test files in parallel. Use --no-file-parallelism to disable (default: true)',
          },
          {
            label: '--maxWorkers <WORKERS>',
            description: 'Maximum number or percentage of workers to run tests in',
          },
          {
            label: '--environment <NAME>',
            description: 'Specify runner environment (default: node)',
          },
          { label: '--passWithNoTests', description: 'Pass when no tests are found' },
          {
            label: '--logHeapUsage',
            description: 'Show the size of the heap for each test when running in Node.js',
          },
          {
            label: '--detectAsyncLeaks',
            description: 'Detect asynchronous resources leaking from test files (default: false)',
          },
          {
            label: '--allowOnly',
            description: 'Allow tests and suites marked as only (default: !process.env.CI)',
          },
          {
            label: '--dangerouslyIgnoreUnhandledErrors',
            description: 'Ignore any unhandled errors that occur',
          },
          {
            label: '--shard <SHARDS>',
            description: 'Test suite shard to execute in the format <index>/<count>',
          },
          {
            label: '--changed [SINCE]',
            description: 'Run tests affected by changed files (default: false)',
          },
          {
            label: '--sequence <OPTIONS>',
            description: 'Configure test sorting',
          },
          {
            label: '--inspect [[HOST:]PORT]',
            description: 'Enable Node.js inspector (default: 127.0.0.1:9229)',
          },
          {
            label: '--inspectBrk [[HOST:]PORT]',
            description: 'Enable Node.js inspector and break before tests start',
          },
          {
            label: '--testTimeout <TIMEOUT>',
            description: 'Default test timeout in milliseconds (default: 5000; 0 disables)',
          },
          {
            label: '--hookTimeout <TIMEOUT>',
            description: 'Default hook timeout in milliseconds (default: 10000; 0 disables)',
          },
          {
            label: '--bail <NUMBER>',
            description: 'Stop test execution after the given number of failures (default: 0)',
          },
          {
            label: '--retry <TIMES>',
            description: 'Retry failed tests (default: 0)',
          },
          {
            label: '--diff <PATH>',
            description: 'DiffOptions object or path to a module exporting one',
          },
          {
            label: '--exclude <GLOB>',
            description: 'Additional file globs to exclude from tests',
          },
          {
            label: '--expandSnapshotDiff',
            description: 'Show the full diff when a snapshot fails',
          },
          {
            label: '--disableConsoleIntercept',
            description: 'Disable automatic interception of console logging (default: false)',
          },
          {
            label: '--typecheck',
            description: 'Enable typechecking alongside tests (default: false)',
          },
          {
            label: '--project <NAME>',
            description: 'Select one or more Vitest workspace projects by name or wildcard',
          },
          {
            label: '--slowTestThreshold <THRESHOLD>',
            description: 'Threshold for a test or suite to be considered slow (default: 300ms)',
          },
          {
            label: '--teardownTimeout <TIMEOUT>',
            description: 'Default teardown timeout in milliseconds (default: 10000)',
          },
          {
            label: '--cache',
            description: 'Enable cache',
          },
          {
            label: '--maxConcurrency <NUMBER>',
            description: 'Maximum number of concurrent tests and suites (default: 5)',
          },
          {
            label: '--expect',
            description: 'Configure expect matchers',
          },
          { label: '--printConsoleTrace', description: 'Always print console stack traces' },
          {
            label: '--includeTaskLocation',
            description: 'Collect test and suite locations in the location property',
          },
          {
            label: '--attachmentsDir <DIR>',
            description:
              'Directory for attachments created with context.annotate (default: .vitest-attachments)',
          },
          { label: '--run', description: 'Disable watch mode' },
          {
            label: '--no-color',
            description: 'Remove colors from console output (default: true)',
          },
          {
            label: '--clearScreen',
            description: 'Clear the terminal when rerunning tests in watch mode (default: true)',
          },
          {
            label: '--configLoader <LOADER>',
            description:
              'Use bundle to bundle the config or runner to process it on the fly (default: bundle)',
          },
          {
            label: '--standalone',
            description: 'Start Vitest without running tests until files change (default: false)',
          },
          {
            label: '--mergeReports [PATH]',
            description: 'Merge previously recorded blob reports without running tests',
          },
          {
            label: '--listTags [TYPE]',
            description: 'List available tags; --list-tags=json outputs JSON',
          },
          {
            label: '--clearCache',
            description: 'Delete all Vitest caches without running tests',
          },
          {
            label: '--tagsFilter <EXPRESSION>',
            description: 'Run only tests matching the tag expression',
          },
          {
            label: '--strictTags',
            description: 'Error when a test uses an undefined tag (default: true)',
          },
          {
            label: '--experimental <FEATURES>',
            description: 'Enable experimental features',
          },
          { label: '-h, --help', description: 'Display this message' },
        ],
      },
      {
        title: 'Bench Options',
        rows: [
          {
            label: '--compare <FILENAME>',
            description: 'Benchmark output file to compare against',
          },
          { label: '--outputJson <FILENAME>', description: 'Benchmark output file' },
        ],
      },
      {
        title: 'List Options',
        rows: [
          {
            label: '--json [TRUE/PATH]',
            description: 'Print collected tests as JSON or write to a file (default: false)',
          },
          { label: '--filesOnly', description: 'Print only test files without test cases' },
          {
            label: '--staticParse',
            description: 'Parse files statically instead of running them (default: false)',
          },
          {
            label: '--staticParseConcurrency <LIMIT>',
            description: 'Number of test files to process concurrently',
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
        title: 'Arguments',
        rows: [{ label: '[PATH]...', description: 'Files or directories to lint' }],
      },
      {
        title: 'Basic Configuration',
        rows: [
          { label: '-c, --config <PATH>', description: 'Oxlint configuration file' },
          {
            label: '--tsconfig <PATH>',
            description: 'Override the TypeScript config used for import resolution',
          },
          {
            label: '--init',
            description: 'Initialize Oxlint configuration with default values',
          },
        ],
      },
      {
        title: 'Rule Severity',
        rows: [
          { label: '-A, --allow <NAME>', description: 'Allow a rule or category' },
          { label: '-W, --warn <NAME>', description: 'Emit a warning for a rule or category' },
          { label: '-D, --deny <NAME>', description: 'Emit an error for a rule or category' },
        ],
      },
      {
        title: 'Plugins',
        rows: [
          {
            label: '--disable-unicorn-plugin',
            description: 'Disable the unicorn plugin, which is enabled by default',
          },
          {
            label: '--disable-oxc-plugin',
            description: 'Disable Oxc-specific rules, which are enabled by default',
          },
          {
            label: '--disable-typescript-plugin',
            description: 'Disable the TypeScript plugin, which is enabled by default',
          },
          { label: '--import-plugin', description: 'Enable the import plugin' },
          { label: '--react-plugin', description: 'Enable the React plugin' },
          { label: '--jsdoc-plugin', description: 'Enable the JSDoc plugin' },
          { label: '--jest-plugin', description: 'Enable the Jest plugin' },
          { label: '--vitest-plugin', description: 'Enable the Vitest plugin' },
          { label: '--jsx-a11y-plugin', description: 'Enable the JSX accessibility plugin' },
          { label: '--nextjs-plugin', description: 'Enable the Next.js plugin' },
          { label: '--react-perf-plugin', description: 'Enable the React performance plugin' },
          { label: '--promise-plugin', description: 'Enable the promise plugin' },
          { label: '--node-plugin', description: 'Enable the Node.js plugin' },
          { label: '--vue-plugin', description: 'Enable the Vue plugin' },
        ],
      },
      {
        title: 'Fix Problems',
        rows: [
          { label: '--fix', description: 'Fix issues when possible' },
          { label: '--fix-suggestions', description: 'Apply auto-fixable suggestions' },
          { label: '--fix-dangerously', description: 'Apply dangerous fixes and suggestions' },
        ],
      },
      {
        title: 'Ignore Files',
        rows: [
          { label: '--ignore-path <PATH>', description: 'Use the specified .eslintignore file' },
          {
            label: '--ignore-pattern <PATTERN>',
            description: 'Add file patterns to ignore',
          },
          { label: '--no-ignore', description: 'Disable file exclusion from ignore rules' },
        ],
      },
      {
        title: 'Handle Warnings',
        rows: [
          { label: '--quiet', description: 'Report errors only' },
          { label: '--deny-warnings', description: 'Exit non-zero when warnings are reported' },
          {
            label: '--max-warnings <INT>',
            description: 'Set the warning threshold before exiting non-zero',
          },
        ],
      },
      {
        title: 'Output',
        rows: [
          {
            label: '-f, --format <FORMAT>',
            description:
              'Set output format: checkstyle, default, agent, github, gitlab, json, junit, sarif, stylish, or unix',
          },
          {
            label: '--debug <OPTIONS>',
            description: 'Enable comma-separated debug output options: files or timings',
          },
        ],
      },
      {
        title: 'Miscellaneous',
        rows: [
          { label: '--silent', description: 'Do not display diagnostics' },
          {
            label: '--no-error-on-unmatched-pattern',
            description: 'Do not exit with an error when no files are selected for linting',
          },
          {
            label: '--threads <INT>',
            description: 'Number of threads to use; set to 1 to use one CPU core',
          },
          {
            label: '--print-config',
            description: 'Print the resolved configuration without linting',
          },
        ],
      },
      {
        title: 'Inline Configuration',
        rows: [
          {
            label: '--report-unused-disable-directives',
            description: 'Report unused oxlint-disable directives',
          },
          {
            label: '--report-unused-disable-directives-severity <SEVERITY>',
            description: 'Report unused disable directives at the specified severity',
          },
        ],
      },
      {
        title: 'Options',
        rows: [
          { label: '--rules', description: 'List all registered rules' },
          { label: '--lsp', description: 'Start the language server' },
          {
            label: '--disable-nested-config',
            description: 'Disable automatic loading of nested configuration files',
          },
          { label: '--type-aware', description: 'Enable rules requiring type information' },
          {
            label: '--type-check',
            description: 'Enable experimental type checking and compiler diagnostics',
          },
          { label: '-h, --help', description: 'Print help information' },
          { label: '-V, --version', description: 'Print version information' },
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
        title: 'Arguments',
        rows: [
          {
            label: '[PATH]...',
            description: 'Files, directories, or glob patterns (default: current directory)',
          },
        ],
      },
      {
        title: 'Mode Options',
        rows: [
          { label: '--init', description: 'Initialize .oxfmtrc.json with default values' },
          {
            label: '--migrate <SOURCE>',
            description: 'Migrate configuration from Prettier or Biome to .oxfmtrc.json',
          },
          { label: '--lsp', description: 'Start the language server' },
          {
            label: '--stdin-filepath <PATH>',
            description: 'Specify the file name used to infer the parser for stdin',
          },
        ],
      },
      {
        title: 'Config Options',
        rows: [
          { label: '-c, --config <PATH>', description: 'Path to the configuration file' },
          {
            label: '--disable-nested-config',
            description: 'Do not search for configuration files in subdirectories',
          },
        ],
      },
      {
        title: 'Output Options',
        rows: [
          { label: '--write', description: 'Format and write files in place' },
          {
            label: '--check',
            description: 'Check whether files are formatted and show statistics',
          },
          { label: '--list-different', description: 'List files that would be changed' },
        ],
      },
      {
        title: 'Ignore Options',
        rows: [
          {
            label: '--ignore-path <PATH>',
            description: 'Path to an ignore file; may be specified multiple times',
          },
          {
            label: '--with-node-modules',
            description: 'Format files in node_modules, which are skipped by default',
          },
        ],
      },
      {
        title: 'Runtime Options',
        rows: [
          {
            label: '--no-error-on-unmatched-pattern',
            description: 'Do not exit with an error when the pattern is unmatched',
          },
          {
            label: '--threads <INT>',
            description: 'Number of threads to use; set to 1 to use one CPU core',
          },
        ],
      },
      {
        title: 'Options',
        rows: [
          { label: '-h, --help', description: 'Print help information' },
          { label: '-V, --version', description: 'Print version information' },
        ],
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
    usage: 'vp pack [...FILES] [OPTIONS]',
    summary: ['Build a library.', 'Options are forwarded to Vite+ Pack.'],
    sections: [
      {
        title: 'Arguments',
        rows: [{ label: '[...FILES]', description: 'Files to bundle' }],
      },
      {
        title: 'Options',
        rows: [
          {
            label: '--config-loader <LOADER>',
            description: 'Set the config loader: auto, native, tsx, or unrun (default: auto)',
          },
          { label: '--no-config', description: 'Disable the config file' },
          {
            label: '-f, --format <FORMAT>',
            description: 'Bundle format: esm, cjs, iife, umd (default: esm)',
          },
          { label: '--clean', description: 'Clean output directory, --no-clean to disable' },
          {
            label: '--deps.never-bundle <MODULE>',
            description: 'Mark dependencies as external',
          },
          { label: '--minify', description: 'Minify output' },
          { label: '--devtools', description: 'Enable devtools integration' },
          { label: '--debug [FEAT]', description: 'Show debug logs' },
          {
            label: '--target <TARGET>',
            description: 'Bundle target, e.g "es2015", "esnext"',
          },
          {
            label: '-l, --logLevel <LEVEL>',
            description: 'Set log level: info, warn, error, silent',
          },
          { label: '--fail-on-warn', description: 'Fail on warnings (default: true)' },
          {
            label: '--no-write',
            description:
              'Disable writing files to disk, incompatible with watch mode (default: true)',
          },
          { label: '-d, --out-dir <DIR>', description: 'Output directory (default: dist)' },
          { label: '--treeshake', description: 'Tree-shake bundle (default: true)' },
          { label: '--sourcemap', description: 'Generate source map (default: false)' },
          { label: '--shims', description: 'Enable cjs and esm shims (default: false)' },
          { label: '--platform <PLATFORM>', description: 'Target platform (default: node)' },
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
          { label: '-w, --watch [PATH]', description: 'Watch mode' },
          { label: '--ignore-watch <PATH>', description: 'Ignore custom paths in watch mode' },
          { label: '--from-vite [VITEST]', description: 'Reuse config from Vite or Vitest' },
          { label: '--report', description: 'Size report (default: true)' },
          {
            label: '--env.* <VALUE>',
            description: 'Define compile-time env variables',
          },
          {
            label: '--env-file <FILE>',
            description:
              'Load environment variables from a file, when used together with --env, variables in --env take precedence',
          },
          {
            label: '--env-prefix <PREFIX>',
            description:
              'Prefix for env variables to inject into the bundle (default: VITE_PACK_,TSDOWN_)',
          },
          { label: '--on-success <COMMAND>', description: 'Command to run on success' },
          { label: '--copy <DIR>', description: 'Copy files to output dir' },
          { label: '--public-dir <DIR>', description: 'Alias for --copy, deprecated' },
          { label: '--tsconfig <TSCONFIG>', description: 'Set tsconfig path' },
          { label: '--unbundle', description: 'Unbundle mode' },
          { label: '--root <DIR>', description: 'Root directory of input files' },
          { label: '--exe', description: 'Bundle as executable' },
          { label: '-W, --workspace [DIR]', description: 'Enable workspace mode' },
          {
            label: '-F, --filter <PATTERN>',
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
  const terminatorIndex = commandArgs.indexOf('--');
  const ownArgs = terminatorIndex === -1 ? commandArgs : commandArgs.slice(0, terminatorIndex);
  const hasHelpFlag = ownArgs.some((arg) => arg === '-h' || arg === '--help');
  if (!hasHelpFlag) {
    return false;
  }

  // Arguments after the task/command belong to the wrapped process, not to vp.
  if (command === 'run' || command === 'exec' || command === 'cache') {
    if (commandArgs.length !== 1) {
      return false;
    }
  }

  printHeader();
  log(renderCliDoc(doc));
  return true;
}
