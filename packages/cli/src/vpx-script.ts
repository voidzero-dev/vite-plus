/**
 * Script detection for the project-local `vpx` bin. Mirrors the global CLI in
 * `crates/vp_global_cli/src/commands/vpx_script.rs`; keep the two in sync.
 */

import { accessSync, closeSync, constants, openSync, readSync, statSync } from 'node:fs';
import path from 'node:path';

export const SCRIPT_EXTENSIONS = ['.ts', '.mts', '.cts', '.tsx', '.js', '.mjs', '.cjs', '.jsx'];
const TS_EXTENSIONS = new Set(['.ts', '.mts', '.cts', '.tsx']);

/** Node.js options that take a required value, which may come as the next argument (best effort). */
export const NODE_OPTIONS_WITH_VALUE = new Set([
  '-C',
  '-e',
  '-p',
  '-r',
  '--allow-fs-read',
  '--allow-fs-write',
  '--build-sea',
  '--build-snapshot-config',
  '--conditions',
  '--cpu-prof-dir',
  '--cpu-prof-interval',
  '--cpu-prof-name',
  '--debug-port',
  '--diagnostic-dir',
  '--disable-proto',
  '--disable-warning',
  '--dns-result-order',
  '--env-file',
  '--env-file-if-exists',
  '--eval',
  '--experimental-config-file',
  '--experimental-default-type',
  '--experimental-loader',
  '--experimental-package-map',
  '--experimental-sea-config',
  '--experimental-test-isolation',
  '--experimental-test-tag-filter',
  '--heap-prof-dir',
  '--heap-prof-interval',
  '--heap-prof-name',
  '--heapsnapshot-near-heap-limit',
  '--heapsnapshot-signal',
  '--icu-data-dir',
  '--import',
  '--input-type',
  '--inspect-port',
  '--inspect-publish-uid',
  '--loader',
  '--localstorage-file',
  '--max-http-header-size',
  '--max-old-space-size-percentage',
  '--network-family-autoselection-attempt-timeout',
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
  '--test-coverage-branches',
  '--test-coverage-exclude',
  '--test-coverage-functions',
  '--test-coverage-include',
  '--test-coverage-lines',
  '--test-global-setup',
  '--test-isolation',
  '--test-name-pattern',
  '--test-random-seed',
  '--test-reporter',
  '--test-reporter-destination',
  '--test-rerun-failures',
  '--test-shard',
  '--test-skip-pattern',
  '--test-timeout',
  '--title',
  '--tls-cipher-list',
  '--tls-keylog',
  '--trace-event-categories',
  '--trace-event-file-pattern',
  '--trace-require-module',
  '--unhandled-rejections',
  '--use-largepages',
  '--v8-pool-size',
  '--watch-kill-signal',
  '--watch-path',
]);

export interface VpxFlags {
  packages: string[];
  shellMode: boolean;
  silent: boolean;
  tsconfig?: string;
  help: boolean;
  version: boolean;
}

/** npx-style: flags come before the first positional or unknown flag. */
export function parseVpxArgs(args: string[]): { flags: VpxFlags; positional: string[] } {
  const flags: VpxFlags = {
    packages: [],
    shellMode: false,
    silent: false,
    help: false,
    version: false,
  };
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
    } else if (arg === '-v' || arg === '--version') {
      flags.version = true;
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
    if (arg === '--' || arg === '-' || !arg.startsWith('-')) {
      const script = arg === '--' ? positional[index + 1] : arg;
      if (script !== undefined) {
        checkScriptAfterOptions(script, cwd);
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

/**
 * After Node.js options, only a missing explicit path is an error. A bare name may be
 * the value of an option missing from `NODE_OPTIONS_WITH_VALUE`, such as a coverage
 * glob, and this position never falls back to a package, so Node.js reports a missing
 * one itself.
 */
function checkScriptAfterOptions(token: string, cwd: string): void {
  if (
    isExplicitPath(token) &&
    SCRIPT_EXTENSIONS.some((extension) => token.endsWith(extension)) &&
    !isFile(path.resolve(cwd, token))
  ) {
    throw missingScript(token, path.resolve(cwd, token));
  }
}

/** The error for a script path that is not a file. */
function missingScript(token: string, file: string): ScriptError {
  return new ScriptError(
    statSync(file, { throwIfNoEntry: false })?.isDirectory()
      ? `Script is a directory: ${token}`
      : `Script not found: ${token}`,
  );
}

function isScript(token: string, cwd: string): boolean {
  const file = path.resolve(cwd, token);
  const extension = SCRIPT_EXTENSIONS.find((candidate) => token.endsWith(candidate));
  if (extension) {
    if (isFile(file)) {
      return true;
    }
    if (isExplicitPath(token) || TS_EXTENSIONS.has(extension)) {
      throw missingScript(token, file);
    }
    return false;
  }
  if (!isExplicitPath(token)) {
    return false;
  }
  // A missing explicit path never falls through to a package download.
  const stats = statSync(file, { throwIfNoEntry: false });
  if (!stats) {
    throw new ScriptError(`Script not found: ${token}`);
  }
  return stats.isFile() && hasVpxShebang(file);
}

export function isExplicitPath(token: string): boolean {
  if (token.startsWith('./') || token.startsWith('../') || token.startsWith('/')) {
    return true;
  }
  return (
    process.platform === 'win32' &&
    (token.startsWith('.\\') || token.startsWith('..\\') || /^(?:[a-zA-Z]:[\\/]|\\\\)/.test(token))
  );
}

export function isFile(file: string): boolean {
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

/** Whether a package spec names a version, as in `eslint@9` or `@vue/cli@5`. */
export function hasVersionSpec(spec: string): boolean {
  if (spec.startsWith('@')) {
    const slash = spec.indexOf('/');
    return slash !== -1 && spec.slice(slash + 1).includes('@');
  }
  return spec.includes('@');
}

/** The binary name a package spec runs: `eslint@9` → `eslint`, `@vue/cli@5` → `cli`. */
export function commandName(spec: string): string {
  if (spec.startsWith('@')) {
    const slash = spec.indexOf('/');
    return slash === -1 ? spec : spec.slice(slash + 1).split('@')[0];
  }
  return spec.split('@')[0];
}

/**
 * The nearest `node_modules/.bin/<name>` from `cwd` up (`.cmd` on Windows), like the
 * global CLI's `find_local_binary`. `vp exec` gets this path, not the name: it only
 * searches the package and workspace roots and then `PATH`, which can pick another
 * binary, or none.
 */
export function findLocalBin(name: string, cwd: string): string | undefined {
  const file = process.platform === 'win32' ? `${name}.cmd` : name;
  for (let dir = cwd; ; dir = path.dirname(dir)) {
    const bin = path.join(dir, 'node_modules', '.bin', file);
    if (isFile(bin)) {
      return bin;
    }
    if (path.dirname(dir) === dir) {
      return undefined;
    }
  }
}

function isExecutable(file: string): boolean {
  if (process.platform === 'win32') {
    return true;
  }
  try {
    accessSync(file, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

export type PackageInvocation =
  | { command: 'exec' | 'dlx'; args: string[] }
  | { error: string; note: string };

/**
 * How the project-local bin runs `vpx <pkg> [args...]` with this package's `vp`. A
 * binary installed in `node_modules/.bin`, or an existing executable file, runs with
 * `vp exec`; a versioned package, `--package`, `--shell-mode`, or a local package
 * directory runs with `vp dlx`. Global packages and `PATH` are not searched, so a
 * binary that is not installed is an error rather than a download.
 */
export function packageInvocation(
  flags: VpxFlags,
  [spec, ...rest]: [string, ...string[]],
  cwd: string,
): PackageInvocation {
  const silent = flags.silent ? ['--silent'] : [];
  if (flags.packages.length > 0 || flags.shellMode || hasVersionSpec(spec)) {
    return {
      command: 'dlx',
      args: [
        ...flags.packages.flatMap((name) => ['--package', name]),
        ...(flags.shellMode ? ['--shell-mode'] : []),
        ...silent,
        spec,
        ...rest,
      ],
    };
  }
  if (isExplicitPath(spec)) {
    const file = path.resolve(cwd, spec);
    if (!isFile(file)) {
      return { command: 'dlx', args: [...silent, spec, ...rest] };
    }
    if (!isExecutable(file)) {
      return {
        error: `'${spec}' is not executable`,
        note: `Make it executable with \`chmod +x ${spec}\`.`,
      };
    }
    return { command: 'exec', args: [spec, ...rest] };
  }
  const name = commandName(spec);
  const bin = findLocalBin(name, cwd);
  if (bin) {
    return { command: 'exec', args: [bin, ...rest] };
  }
  return {
    error: `'${name}' is not installed in node_modules`,
    note: `Add it to devDependencies, or run \`vp dlx ${spec}\` to download and run it.`,
  };
}
