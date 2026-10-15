/**
 * Project-local `vpx` bin (`node_modules/.bin/vpx`).
 *
 * Package managers and `vp run` put `node_modules/.bin` first on `PATH`, so this bin,
 * not the global `vpx`, runs in package.json scripts. It never hands off to the global
 * CLI. Script files run with the loader shipped in this package. A binary installed in
 * `node_modules/.bin` runs directly; a versioned package, `--package`, or
 * `--shell-mode` runs with this package's `vp dlx`. See `packageInvocation`.
 *
 * Only `vp dlx` loads the CLI bundle and the native binding. A script's own Node.js
 * process loads the binding through `script-preload.cjs`.
 */

import { statSync } from 'node:fs';
import { createRequire } from 'node:module';
import { constants } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { styleText } from 'node:util';

import spawn from 'cross-spawn';

import {
  detectScript,
  isFile,
  packageInvocation,
  parseVpxArgs,
  ScriptError,
} from './vpx-script.ts';

function errorMsg(message: string) {
  /* oxlint-disable-next-line no-console */
  console.error(styleText(['red', 'bold'], 'error:'), message);
}

function pathKey(env: NodeJS.ProcessEnv): string {
  return Object.keys(env).find((key) => key.toUpperCase() === 'PATH') ?? 'PATH';
}

/** Prepend every `node_modules/.bin` from `cwd` up, nearest first, like the global CLI. */
function prependNodeModulesBins(cwd: string, pathValue: string | undefined): string {
  const entries = (pathValue ?? '').split(path.delimiter).filter(Boolean);
  const bins: string[] = [];
  for (let dir = cwd; ; dir = path.dirname(dir)) {
    const bin = path.join(dir, 'node_modules', '.bin');
    if (statSync(bin, { throwIfNoEntry: false })?.isDirectory() && !entries.includes(bin)) {
      bins.push(bin);
    }
    if (path.dirname(dir) === dir) {
      break;
    }
  }
  return [...bins, ...entries].join(path.delimiter);
}

/**
 * Run `command` with inherited stdio and exit the way it exited. cross-spawn runs a
 * Windows `.cmd` shim from `node_modules/.bin`, which Node.js refuses to spawn directly.
 */
function runAndExit(command: string, args: string[], env: NodeJS.ProcessEnv): void {
  const child = spawn(command, args, { stdio: 'inherit', env });
  // Terminal Ctrl+C reaches the child through the process group; wait for it.
  process.on('SIGINT', () => {});
  for (const signal of ['SIGTERM', 'SIGHUP'] as const) {
    process.on(signal, () => child.kill(signal));
  }
  child.on('error', (error) => {
    errorMsg(`vpx: Failed to run ${command}: ${error.message}`);
    process.exit(1);
  });
  child.on('exit', (code, signal) => {
    if (signal && process.platform !== 'win32') {
      process.removeAllListeners(signal);
      process.kill(process.pid, signal);
      return;
    }
    process.exit(code ?? (signal ? 128 + (constants.signals[signal] ?? 0) : 1));
  });
}

const LOCAL_HELP = `Execute a binary from node_modules/.bin or a remote npm package, or run a script file

Usage: vpx [OPTIONS] <pkg[@version]> [args...]
       vpx [OPTIONS] [NODE_OPTIONS] <script> [args...]

Arguments:
  <pkg[@version]>  Package binary to execute
  <script>         Script file to run (.ts, .mts, .cts, .tsx, .js, .mjs, .cjs, .jsx)
  [args...]        Arguments to pass to the command or script

Options:
  -p, --package <NAME>  Package(s) to download
  -c, --shell-mode      Execute the command within a shell environment
  -s, --silent          Suppress all output except the command's output
      --tsconfig <PATH> tsconfig.json to use when running a script
  -v, --version         Print the Vite+ version
  -h, --help            Print help

This project-local vpx runs in package.json scripts. It runs binaries installed in
node_modules/.bin, and downloads a package only for a version, --package, or --shell-mode.

Examples:
  vpx eslint .                           # Run eslint from node_modules/.bin
  vpx create-vue@latest my-app           # Download and run create-vue
  vpx -p cowsay -c 'echo "hi" | cowsay'  # Shell mode with package
  vpx ./scripts/seed.ts --dry-run        # Run a TypeScript script
  vpx --watch ./server.ts                # Pass Node.js options before the script
`;

function fail(message: string): never {
  errorMsg(message);
  process.exit(1);
}

const args = process.argv.slice(2);
const cwd = process.cwd();
const { flags, positional } = parseVpxArgs(args);

if (flags.help) {
  process.stdout.write(LOCAL_HELP);
  process.exit(0);
}
if (flags.version) {
  const { version } = createRequire(import.meta.url)('../package.json') as { version: string };
  process.stdout.write(`vite-plus v${version}\n`);
  process.exit(0);
}
const [first, ...rest] = positional;
if (first === undefined) {
  errorMsg('vpx requires a command to run');
  process.stderr.write(
    '\nUsage: vpx <pkg[@version]> [args...]\n       vpx <script> [args...]\n\n' +
      'Examples:\n  vpx eslint .\n  vpx create-vue my-app\n  vpx ./scripts/seed.ts\n',
  );
  process.exit(1);
}

// Script mode is decided before package-spec parsing: script paths can contain `@`.
const packageFlags = flags.packages.length > 0 || flags.shellMode;
let invocation;
if (!packageFlags || !first.startsWith('-')) {
  try {
    invocation = detectScript(positional, cwd);
  } catch (error) {
    if (!(error instanceof ScriptError)) {
      throw error;
    }
    // A missing script never falls through to `dlx`, `--package` or not. With
    // `--shell-mode` the token is a shell command, not a path.
    if (!flags.shellMode) {
      fail(`vpx: ${error.message}`);
    }
  }
  if (invocation && packageFlags) {
    fail(
      `vpx: ${flags.shellMode ? '--shell-mode' : '--package'} cannot be used when running a script`,
    );
  }
}

if (invocation) {
  const env = { ...process.env };
  const key = pathKey(env);
  env[key] = prependNodeModulesBins(cwd, env[key]);
  const tsconfig = invocation.tsconfig ?? flags.tsconfig;
  if (tsconfig === undefined) {
    delete env.VP_SCRIPT_TSCONFIG;
  } else {
    const file = path.resolve(cwd, tsconfig);
    if (!isFile(file)) {
      fail(`vpx: tsconfig not found: ${tsconfig}`);
    }
    env.VP_SCRIPT_TSCONFIG = file;
  }
  // The preload registers the hooks before any user `--require`; the `--import`
  // registers the off-thread hooks on older Node.js and routes the entry point
  // through the ESM loader.
  runAndExit(
    process.execPath,
    [
      '--require',
      fileURLToPath(new URL('./script-preload.cjs', import.meta.url)),
      '--import',
      new URL('./script-register.js', import.meta.url).href,
      ...invocation.nodeArgs,
    ],
    env,
  );
} else {
  if (flags.tsconfig !== undefined) {
    fail('vpx: --tsconfig can only be used when running a script');
  }
  const result = packageInvocation(flags, [first, ...rest], cwd);
  if ('error' in result) {
    errorMsg(`vpx: ${result.error}`);
    if (result.note) {
      /* oxlint-disable-next-line no-console */
      console.error(styleText(['dim', 'bold'], 'note:'), result.note);
    }
    process.exit(1);
  }
  if (result.command === 'run') {
    // As the global CLI runs a local binary: the file itself, with every
    // `node_modules/.bin` from the cwd up first on PATH.
    const env = { ...process.env };
    const key = pathKey(env);
    env[key] = prependNodeModulesBins(cwd, env[key]);
    runAndExit(result.file, result.args, env);
  } else {
    // Hand off to this package's `vp dlx` in this process. A computed specifier
    // keeps the CLI bundle and the native binding out of this entry.
    process.argv = [...process.argv.slice(0, 2), 'dlx', ...result.args];
    const cliEntry = new URL('./bin.js', import.meta.url).href;
    await import(cliEntry);
  }
}
