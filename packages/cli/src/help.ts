import { renderCliDoc, type CliDoc } from './utils/help.ts';
import { log, printHeader } from './utils/terminal.ts';

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
          { label: '--base <PATH>', description: 'Public base path' },
          { label: '-l, --logLevel <LEVEL>', description: 'Set log level' },
          { label: '--clearScreen', description: 'Allow or disable clearing the screen' },
          { label: '--configLoader <LOADER>', description: 'Set the config loader' },
          { label: '-d, --debug [FEAT]', description: 'Show debug logs' },
          { label: '-f, --filter <FILTER>', description: 'Filter debug logs' },
          { label: '-m, --mode <MODE>', description: 'Set env mode' },
          { label: '-h, --help', description: 'Print help' },
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
          { label: '--base <PATH>', description: 'Public base path' },
          { label: '-l, --logLevel <LEVEL>', description: 'Set log level' },
          { label: '--clearScreen', description: 'Allow or disable clearing the screen' },
          { label: '--configLoader <LOADER>', description: 'Set the config loader' },
          { label: '-d, --debug [FEAT]', description: 'Show debug logs' },
          { label: '-f, --filter <FILTER>', description: 'Filter debug logs' },
          { label: '-m, --mode <MODE>', description: 'Set env mode' },
          { label: '-h, --help', description: 'Print help' },
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
          { label: '--base <PATH>', description: 'Public base path' },
          { label: '-l, --logLevel <LEVEL>', description: 'Set log level' },
          { label: '--clearScreen', description: 'Allow or disable clearing the screen' },
          { label: '--configLoader <LOADER>', description: 'Set the config loader' },
          { label: '-d, --debug [FEAT]', description: 'Show debug logs' },
          { label: '-f, --filter <FILTER>', description: 'Filter debug logs' },
          { label: '-m, --mode <MODE>', description: 'Set env mode' },
          { label: '-h, --help', description: 'Print help' },
        ],
      },
      { title: 'Examples', lines: ['  vp preview', '  vp preview --port 4173'] },
    ],
    documentationUrl: 'https://viteplus.dev/guide/build',
  },
  test: {
    usage: 'vp test [COMMAND] [FILTERS] [OPTIONS]',
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
          { label: 'init', description: 'Initialize Vitest config' },
          { label: 'list', description: 'List matching tests' },
        ],
      },
      {
        title: 'Options',
        rows: [
          { label: '-r, --root <PATH>', description: 'Set the project root' },
          { label: '-u, --update [TYPE]', description: 'Update snapshots' },
          { label: '-w, --watch', description: 'Enable watch mode' },
          { label: '-t, --testNamePattern <PATTERN>', description: 'Run tests matching regexp' },
          { label: '--dir <PATH>', description: 'Set the directory to scan for tests' },
          { label: '--ui', description: 'Enable UI' },
          { label: '--open', description: 'Open UI automatically' },
          { label: '--coverage', description: 'Enable coverage' },
          { label: '--reporter <NAME>', description: 'Specify reporter' },
          { label: '--browser <NAME>', description: 'Run tests in the browser' },
          { label: '--pool <POOL>', description: 'Set the worker pool' },
          { label: '--maxWorkers <WORKERS>', description: 'Set the maximum number of workers' },
          { label: '--environment <NAME>', description: 'Set the test environment' },
          { label: '--passWithNoTests', description: 'Pass when no tests are found' },
          { label: '--run', description: 'Disable watch mode' },
          { label: '-h, --help', description: 'Print help' },
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
        title: 'Options',
        rows: [
          { label: '--tsconfig <PATH>', description: 'Override the TypeScript config' },
          { label: '--fix', description: 'Fix issues when possible' },
          { label: '--fix-suggestions', description: 'Apply auto-fixable suggestions' },
          { label: '--fix-dangerously', description: 'Apply dangerous fixes and suggestions' },
          { label: '--type-aware', description: 'Enable rules requiring type information' },
          { label: '--type-check', description: 'Enable experimental type checking' },
          { label: '--import-plugin', description: 'Enable the import plugin' },
          { label: '--disable-nested-config', description: 'Disable nested config discovery' },
          {
            label: '--no-error-on-unmatched-pattern',
            description: 'Do not exit with error when no files are selected',
          },
          { label: '--rules', description: 'List registered rules' },
          { label: '-h, --help', description: 'Print help' },
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
        title: 'Options',
        rows: [
          { label: '--write', description: 'Format and write files in place' },
          { label: '--check', description: 'Check if files are formatted' },
          { label: '--list-different', description: 'List files that would be changed' },
          { label: '--disable-nested-config', description: 'Disable nested config discovery' },
          { label: '--ignore-path <PATH>', description: 'Path to ignore file(s)' },
          { label: '--with-node-modules', description: 'Format files in node_modules' },
          {
            label: '--no-error-on-unmatched-pattern',
            description: 'Do not exit with error when no files are selected',
          },
          { label: '--threads <INT>', description: 'Number of threads to use' },
          { label: '-h, --help', description: 'Print help' },
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
        title: 'Options',
        rows: [
          { label: '--config-loader <LOADER>', description: 'Set the config loader' },
          { label: '--no-config', description: 'Disable the config file' },
          { label: '-f, --format <FORMAT>', description: 'Bundle format: esm, cjs, iife, umd' },
          { label: '-d, --out-dir <DIR>', description: 'Output directory' },
          { label: '--target <TARGET>', description: 'Bundle target' },
          { label: '--platform <PLATFORM>', description: 'Target platform' },
          { label: '--sourcemap', description: 'Generate source maps' },
          { label: '--dts', description: 'Generate declaration files' },
          { label: '--minify', description: 'Minify output' },
          { label: '--exe', description: 'Bundle as an executable' },
          { label: '-W, --workspace [DIR]', description: 'Enable workspace mode' },
          { label: '-F, --filter <PATTERN>', description: 'Filter workspace configs' },
          { label: '-w, --watch [PATH]', description: 'Watch mode' },
          { label: '-h, --help', description: 'Print help' },
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
