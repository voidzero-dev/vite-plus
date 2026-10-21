/**
 * Project-local `vpx` bin (`node_modules/.bin/vpx`).
 *
 * Package managers and `vp run` put `node_modules/.bin` first on `PATH`, so this bin,
 * not the global `vpx`, runs in package.json scripts. It never hands off to the global
 * CLI. Script files run with the loader shipped in this package, and `--watch` runs the
 * watcher in `script-watch.ts` in this process. A binary installed in
 * `node_modules/.bin` runs directly; a versioned package, `--package`, or
 * `--shell-mode` runs with this package's `vp dlx`. See `packageInvocation`.
 *
 * Only `vp dlx` loads the CLI bundle and the native binding. A script's own Node.js
 * process loads the binding through `script-preload.cjs`.
 */

import type { Serializable } from 'node:child_process';
import { statSync } from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { styleText } from 'node:util';

import spawn from 'cross-spawn';

import { exitCodeFromClose } from './utils/command.ts';
import {
  canExecDirectly,
  detectScript,
  isFile,
  nodeModulesBinPaths,
  packageInvocation,
  parseVpxArgs,
  ScriptError,
} from './vpx-script.ts';

function errorMsg(message: string) {
  /* oxlint-disable-next-line no-console */
  console.error(styleText(['red', 'bold'], 'error:'), message);
}

/** `env` with every existing `node_modules/.bin` from `cwd` up first on PATH, nearest first, like the global CLI. */
function prependLocalBins(env: NodeJS.ProcessEnv, cwd: string): NodeJS.ProcessEnv {
  const key = Object.keys(env).find((name) => name.toUpperCase() === 'PATH') ?? 'PATH';
  const entries = (env[key] ?? '').split(path.delimiter).filter(Boolean);
  const bins = nodeModulesBinPaths(cwd).filter(
    (bin) => !entries.includes(bin) && statSync(bin, { throwIfNoEntry: false })?.isDirectory(),
  );
  env[key] = [...bins, ...entries].join(path.delimiter);
  return env;
}

/** This process's environment for a child, with `prependLocalBins`. */
function childEnv(cwd: string): NodeJS.ProcessEnv {
  return prependLocalBins({ ...process.env }, cwd);
}

/** Signals Node.js ignores; an ignored signal stays ignored across exec. */
const IGNORED_SIGNALS = ['SIGPIPE', 'SIGXFSZ'] as const;
const ignoreSignal = () => {};

interface StdioStream {
  _handle?: { setBlocking?: (blocking: boolean) => number };
}

/**
 * Replace this process with `command`, as the global `vpx` does on Unix, so the caller's
 * PID, every signal sent to it, and the exit status belong to the script or binary.
 * Returns on Windows, which has no exec, so the caller spawns instead.
 *
 * The new program starts as a spawned child would: libuv gives a child blocking stdio
 * and default signal dispositions, while Node.js ignores SIGPIPE and makes a pipe it
 * opened for stdio non-blocking (loading `node:util` opens stderr), which
 * `process.execve()` passes on (nodejs/node#66824). Removing a signal listener restores
 * the default disposition.
 */
function execInPlace(command: string, args: string[], env: NodeJS.ProcessEnv): void {
  if (process.platform === 'win32' || typeof process.execve !== 'function') {
    return;
  }
  for (const signal of IGNORED_SIGNALS) {
    process.on(signal, ignoreSignal).off(signal, ignoreSignal);
  }
  for (const stream of [process.stdin, process.stdout, process.stderr]) {
    /* oxlint-disable-next-line no-underscore-dangle -- Node.js has no public API for this */
    (stream as StdioStream)._handle?.setBlocking?.(true);
  }
  // A failed exec aborts most supported Node.js versions instead of throwing, so a
  // binary `canExecDirectly` cannot vouch for execs through `/bin/sh`, which reports one
  // it cannot run as a terminal does. Skipping the shell saves its startup.
  const [file, argv] =
    command === process.execPath || canExecDirectly(command)
      ? [command, [command, ...args]]
      : ['/bin/sh', ['/bin/sh', '-c', 'exec "$0" "$@"', command, ...args]];
  try {
    process.execve(file, argv, env);
  } catch {
    for (const signal of IGNORED_SIGNALS) {
      process.on(signal, ignoreSignal);
    }
  }
}

/**
 * Run `command` with inherited stdio and exit the way it exited. On Unix, it execs
 * `command` in place unless this bin has an IPC channel, which exec would close.
 * cross-spawn runs a Windows `.cmd` shim from `node_modules/.bin`, which Node.js refuses
 * to spawn directly.
 *
 * With `relayIpc`, an IPC channel this bin was started with (by `fork()`, or with an
 * `ipc` stdio entry) is relayed to the child in both directions, as tsx does, so
 * `process.send()` in a script reaches the process that started `vpx`. Messages use the
 * default JSON serialization; a message it cannot serialize, or a handle, is not relayed.
 */
function runAndExit(
  command: string,
  args: string[],
  env: NodeJS.ProcessEnv,
  { relayIpc = false } = {},
): void {
  if (process.send === undefined) {
    execInPlace(command, args, env);
  }
  const ipc = relayIpc && process.send !== undefined;
  const child = spawn(command, args, {
    stdio: ipc ? ['inherit', 'inherit', 'inherit', 'ipc'] : 'inherit',
    env,
  });
  // Exit once the child has exited and, with IPC, its channel has closed and every
  // message it sent has been relayed. `close` alone is not enough: it never fires when
  // this side disconnects the channel first.
  let exited: { code: number | null; signal: NodeJS.Signals | null } | undefined;
  let channelClosed = !ipc;
  let pendingSends = 0;
  const exitWhenDone = () => {
    if (!exited || !channelClosed || pendingSends > 0) {
      return;
    }
    const { code, signal } = exited;
    if (signal && process.platform !== 'win32') {
      // Die by the same signal. Removing every listener, and the one added and removed
      // here, restores its default action, which also covers SIGPIPE and SIGXFSZ, which
      // Node.js ignores. The exit below runs only if the signal still does not end us.
      process.removeAllListeners(signal);
      process.on(signal, ignoreSignal).off(signal, ignoreSignal);
      process.kill(process.pid, signal);
    }
    process.exit(exitCodeFromClose(code, signal));
  };
  if (ipc) {
    child.on('message', (message) => {
      pendingSends++;
      process.send!(message, undefined, undefined, () => {
        pendingSends--;
        exitWhenDone();
      });
    });
    // Node.js emits a channel's messages before its `disconnect`, whichever side closed it.
    child.on('disconnect', () => {
      channelClosed = true;
      exitWhenDone();
    });
    process.on('message', (message) => {
      if (!child.connected) {
        return;
      }
      try {
        child.send(message as Serializable);
      } catch (error) {
        errorMsg(`vpx: Failed to relay an IPC message to the script: ${(error as Error).message}`);
      }
    });
    process.on('disconnect', () => {
      if (child.connected) {
        child.disconnect();
      }
    });
  }
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
    exited = { code, signal };
    exitWhenDone();
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

Before a script, --watch reruns it when a file it uses changes (see \`vpx --watch --help\`).

This project-local vpx runs in package.json scripts. It runs binaries installed in
node_modules/.bin, and downloads a package only for a version, --package, or --shell-mode.

Examples:
  vpx eslint .                           # Run eslint from node_modules/.bin
  vpx create-vue@latest my-app           # Download and run create-vue
  vpx -p cowsay -c 'echo "hi" | cowsay'  # Shell mode with package
  vpx ./scripts/seed.ts --dry-run        # Run a TypeScript script
  vpx --inspect ./server.ts              # Pass Node.js options before the script
  vpx --watch ./server.ts                # Rerun the script when a file it uses changes
`;

function fail(message: string, note?: string): never {
  errorMsg(message);
  if (note) {
    /* oxlint-disable-next-line no-console */
    console.error(styleText(['dim', 'bold'], 'note:'), note);
  }
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
  // With `--watch`, the watcher runs in this process, as `vp dlx` does below, and starts
  // the script itself, so it takes this process's environment.
  const env = invocation.watch ? prependLocalBins(process.env, cwd) : childEnv(cwd);
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
  if (invocation.watch) {
    process.argv = [...process.argv.slice(0, 2), ...invocation.nodeArgs];
    await import(new URL('./script-watch.js', import.meta.url).href);
  } else {
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
      { relayIpc: true },
    );
  }
} else {
  if (flags.tsconfig !== undefined) {
    fail('vpx: --tsconfig can only be used when running a script');
  }
  const result = packageInvocation(flags, [first, ...rest], cwd);
  if ('error' in result) {
    fail(`vpx: ${result.error}`, result.note);
  }
  if (result.command === 'run') {
    // As the global CLI runs a local binary: the file itself, with every
    // `node_modules/.bin` from the cwd up first on PATH.
    runAndExit(result.file, result.args, childEnv(cwd));
  } else {
    // Hand off to this package's `vp dlx` in this process. A computed specifier
    // keeps the CLI bundle and the native binding out of this entry.
    process.argv = [...process.argv.slice(0, 2), 'dlx', ...result.args];
    const cliEntry = new URL('./bin.js', import.meta.url).href;
    await import(cliEntry);
  }
}
