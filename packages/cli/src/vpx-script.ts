/**
 * Script detection for the project-local `vpx` bin. Mirrors the global CLI in
 * `crates/vp_global_cli/src/commands/vpx_script.rs`; keep the two in sync.
 */

import { closeSync, openSync, readSync, statSync } from 'node:fs';
import path from 'node:path';

const SCRIPT_EXTENSIONS = ['.ts', '.mts', '.cts', '.tsx', '.js', '.mjs', '.cjs', '.jsx'];
const TS_EXTENSIONS = new Set(['.ts', '.mts', '.cts', '.tsx']);

/** Node.js options that take their value as the next argument (best effort). */
const NODE_OPTIONS_WITH_VALUE = new Set([
  '-C',
  '-e',
  '-p',
  '-r',
  '--conditions',
  '--cpu-prof-dir',
  '--cpu-prof-name',
  '--diagnostic-dir',
  '--disable-warning',
  '--dns-result-order',
  '--env-file',
  '--env-file-if-exists',
  '--eval',
  '--experimental-config-file',
  '--experimental-loader',
  '--heap-prof-dir',
  '--heap-prof-name',
  '--heapsnapshot-signal',
  '--icu-data-dir',
  '--import',
  '--input-type',
  '--inspect-port',
  '--loader',
  '--localstorage-file',
  '--openssl-config',
  '--print',
  '--redirect-warnings',
  '--report-dir',
  '--report-directory',
  '--report-filename',
  '--report-signal',
  '--require',
  '--run',
  '--secure-heap',
  '--secure-heap-min',
  '--snapshot-blob',
  '--test-concurrency',
  '--test-name-pattern',
  '--test-reporter',
  '--test-reporter-destination',
  '--test-shard',
  '--test-skip-pattern',
  '--test-timeout',
  '--title',
  '--tls-cipher-list',
  '--tls-keylog',
  '--unhandled-rejections',
  '--watch-kill-signal',
  '--watch-path',
]);

export interface VpxFlags {
  packages: string[];
  shellMode: boolean;
  silent: boolean;
  tsconfig?: string;
  help: boolean;
}

/** npx-style: flags come before the first positional or unknown flag. */
export function parseVpxArgs(args: string[]): { flags: VpxFlags; positional: string[] } {
  const flags: VpxFlags = { packages: [], shellMode: false, silent: false, help: false };
  let index = 0;
  for (; index < args.length; index++) {
    const arg = args[index];
    if (arg === '-p' || arg === '--package') {
      index++;
      if (index < args.length) {
        flags.packages.push(args[index]);
      }
    } else if (arg.startsWith('--package=') || arg.startsWith('-p=')) {
      flags.packages.push(arg.slice(arg.indexOf('=') + 1));
    } else if (arg === '-c' || arg === '--shell-mode') {
      flags.shellMode = true;
    } else if (arg === '-s' || arg === '--silent') {
      flags.silent = true;
    } else if (arg === '-h' || arg === '--help') {
      flags.help = true;
    } else if (arg === '--tsconfig') {
      index++;
      if (index < args.length) {
        flags.tsconfig = args[index];
      }
    } else if (arg.startsWith('--tsconfig=')) {
      flags.tsconfig = arg.slice('--tsconfig='.length);
    } else {
      break;
    }
  }
  return { flags, positional: args.slice(index) };
}

export interface ScriptInvocation {
  /** Node.js options, the script, and its arguments, in the order given. */
  nodeArgs: string[];
  /** `--tsconfig` given among the Node.js options. */
  tsconfig?: string;
}

export class ScriptError extends Error {}

/**
 * Decide whether `positional` runs a script; `undefined` means package mode.
 * Throws {@link ScriptError} for a missing script, which must never fall
 * through to a remote package download.
 */
export function detectScript(positional: string[], cwd: string): ScriptInvocation | undefined {
  const [first] = positional;
  if (first === undefined) {
    return undefined;
  }
  if (!first.startsWith('-')) {
    return isScript(first, cwd) ? { nodeArgs: [...positional] } : undefined;
  }

  // A leading option never named a package bin, so this always runs Node.js.
  const nodeArgs: string[] = [];
  let tsconfig: string | undefined;
  for (let index = 0; index < positional.length; index++) {
    const arg = positional[index];
    if (arg === '--') {
      if (positional[index + 1] !== undefined) {
        isScript(positional[index + 1], cwd);
      }
      nodeArgs.push(...positional.slice(index));
      break;
    }
    if (arg === '-' || !arg.startsWith('-')) {
      if (arg !== '-') {
        isScript(arg, cwd);
      }
      nodeArgs.push(...positional.slice(index));
      break;
    }
    if (arg === '--tsconfig') {
      if (positional[index + 1] === undefined) {
        throw new ScriptError('--tsconfig requires a path');
      }
      tsconfig = positional[++index];
      continue;
    }
    if (arg.startsWith('--tsconfig=')) {
      tsconfig = arg.slice('--tsconfig='.length);
      continue;
    }
    nodeArgs.push(arg);
    if (NODE_OPTIONS_WITH_VALUE.has(arg) && positional[index + 1] !== undefined) {
      nodeArgs.push(positional[++index]);
    }
  }
  return { nodeArgs, tsconfig };
}

function isScript(token: string, cwd: string): boolean {
  const file = path.resolve(cwd, token);
  const extension = SCRIPT_EXTENSIONS.find((candidate) => token.endsWith(candidate));
  if (extension) {
    if (isFile(file)) {
      return true;
    }
    if (isExplicitPath(token) || TS_EXTENSIONS.has(extension)) {
      throw new ScriptError(`Script not found: ${token}`);
    }
    return false;
  }
  return isExplicitPath(token) && isFile(file) && hasVpxShebang(file);
}

function isExplicitPath(token: string): boolean {
  if (token.startsWith('./') || token.startsWith('../') || token.startsWith('/')) {
    return true;
  }
  return (
    process.platform === 'win32' &&
    (token.startsWith('.\\') || token.startsWith('..\\') || /^(?:[a-zA-Z]:[\\/]|\\\\)/.test(token))
  );
}

function isFile(file: string): boolean {
  return statSync(file, { throwIfNoEntry: false })?.isFile() ?? false;
}

function hasVpxShebang(file: string): boolean {
  const head = Buffer.alloc(256);
  let read = 0;
  try {
    const fd = openSync(file, 'r');
    try {
      read = readSync(fd, head, 0, head.length, 0);
    } finally {
      closeSync(fd);
    }
  } catch {
    return false;
  }
  const text = head.subarray(0, read).toString('utf8');
  if (!text.startsWith('#!')) {
    return false;
  }
  return text
    .slice(2)
    .split('\n', 1)[0]
    .split(/\s+/)
    .some((word) => {
      const name = word.split(/[\\/]/).pop();
      return name === 'vpx' || name === 'vpx.exe';
    });
}
