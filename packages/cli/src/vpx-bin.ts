/**
 * Project-local `vpx` bin (`node_modules/.bin/vpx`).
 *
 * Runs script files with the loader shipped in this package, so `vpx ./seed.ts`
 * works in package.json scripts without the global CLI. Everything else is
 * handed to the global `vpx`, which owns the package lookup chain.
 *
 * Kept free of the CLI bundle and the native binding: the script's own Node.js
 * process loads the binding through `script-register.js`.
 */

import { spawn } from 'node:child_process';
import { realpathSync, statSync } from 'node:fs';
import { constants } from 'node:os';
import path from 'node:path';
import { styleText } from 'node:util';

import { detectScript, parseVpxArgs, ScriptError } from './vpx-script.ts';

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

/** The global `vpx` on PATH, skipping `node_modules/.bin` directories (this bin). */
function findGlobalVpx(env: NodeJS.ProcessEnv): string | undefined {
  const names = process.platform === 'win32' ? ['vpx.exe'] : ['vpx'];
  const self = realpathSync(process.argv[1]);
  for (const dir of (env[pathKey(env)] ?? '').split(path.delimiter)) {
    if (!dir || path.basename(dir) === '.bin') {
      continue;
    }
    for (const name of names) {
      const candidate = path.join(dir, name);
      if (
        statSync(candidate, { throwIfNoEntry: false })?.isFile() &&
        realpathSync(candidate) !== self
      ) {
        return candidate;
      }
    }
  }
  return undefined;
}

/** Run `command` with inherited stdio and exit the way it exited. */
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

const args = process.argv.slice(2);
const cwd = process.cwd();
const { flags, positional } = parseVpxArgs(args);

let invocation;
try {
  invocation =
    flags.packages.length === 0 && !flags.shellMode && !flags.help
      ? detectScript(positional, cwd)
      : undefined;
} catch (error) {
  if (!(error instanceof ScriptError)) {
    throw error;
  }
  errorMsg(`vpx: ${error.message}`);
  process.exit(1);
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
    if (!statSync(file, { throwIfNoEntry: false })?.isFile()) {
      errorMsg(`vpx: tsconfig not found: ${tsconfig}`);
      process.exit(1);
    }
    env.VP_SCRIPT_TSCONFIG = file;
  }
  const loader = new URL('./script-register.js', import.meta.url).href;
  runAndExit(process.execPath, ['--import', loader, ...invocation.nodeArgs], env);
} else {
  const globalVpx = findGlobalVpx(process.env);
  if (!globalVpx) {
    errorMsg(
      'vpx: Running package binaries requires the global Vite+ CLI (https://viteplus.dev/guide/). ' +
        'Use `vp exec` or `vp dlx` from a project-local install.',
    );
    process.exit(1);
  }
  runAndExit(globalVpx, args, process.env);
}
