/**
 * Script detection for the project-local `vpx` bin. Mirrors the global CLI in
 * `crates/vp_global_cli/src/commands/vpx_script.rs`; keep the two in sync.
 */

import {
  accessSync,
  closeSync,
  constants,
  openSync,
  readSync,
  type Stats,
  statSync,
} from 'node:fs';
import path from 'node:path';

export const SCRIPT_EXTENSIONS = ['.ts', '.mts', '.cts', '.tsx', '.js', '.mjs', '.cjs', '.jsx'];
const TS_EXTENSIONS = new Set(['.ts', '.mts', '.cts', '.tsx']);

/** Node.js options that take a required value, which may come as the next argument (best effort). */
export const NODE_OPTIONS_WITH_VALUE = new Set([
  '-C',
  '-e',
  '-p',
  '-pe',
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

/**
 * Node.js options after which positionals are not an entry file: the code of `--eval` /
 * `--print` or the package script of `--run` runs instead, and `--test` takes test files
 * and globs.
 */
export const NODE_OPTIONS_WITHOUT_ENTRY = new Set([
  '-e',
  '--eval',
  '-p',
  '-pe',
  '--print',
  '--run',
  '--test',
]);

/**
 * The watcher's options that take a value, which may come as the next argument, so that
 * value is not taken for the script.
 */
export const WATCH_OPTIONS_WITH_VALUE = new Set([
  '--exclude',
  '--ignore',
  '--include',
  '--kill-signal',
  '--kill-timeout',
]);

/**
 * The option an argument names, without `=value`. Node.js accepts `_` for `-` in long
 * option names, as in `--redirect_warnings`.
 */
function nodeOptionName(arg: string): string {
  const name = arg.split('=', 1)[0];
  return name.startsWith('--') ? `--${name.slice(2).replaceAll('_', '-')}` : name;
}

/**
 * Whether the option `name`, given without `=value`, takes `next` as its value. Node.js
 * 24.19, 26.1, and later read a bare `--experimental-config-file` as `node.config.json`,
 * and older releases take the next argument as the config file, so that argument is its
 * value only when it is a `.json` file, which cannot be the script either way. Mirrors
 * `takes_next_arg` in the global CLI.
 */
function takesNextArg(name: string, next: string): boolean {
  if (name === '--experimental-config-file') {
    return next.endsWith('.json') && !next.startsWith('-');
  }
  return NODE_OPTIONS_WITH_VALUE.has(name) || WATCH_OPTIONS_WITH_VALUE.has(name);
}

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
    } else if (arg === '--tsconfig' && index + 1 < args.length) {
      // Without a value it stays positional, where `detectScript` reports the missing path.
      flags.tsconfig = args[++index];
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
  /**
   * `--watch` or `--watch-path` among the Node.js options: the watcher runs the script,
   * never Node.js' own watch mode. `--watch` is removed from `nodeArgs`; the watcher
   * rejects `--watch-path` with a hint.
   */
  watch?: true;
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
  let watch = false;
  let entry = true;
  for (let index = 0; index < positional.length; index++) {
    const arg = positional[index];
    if (arg === '--' || arg === '-' || !arg.startsWith('-')) {
      const script = arg === '--' ? positional[index + 1] : arg;
      if (script !== undefined && entry) {
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
    if (arg === '--watch') {
      watch = true;
      continue;
    }
    const name = nodeOptionName(arg);
    watch ||= name === '--watch-path';
    entry &&= !NODE_OPTIONS_WITHOUT_ENTRY.has(name);
    nodeArgs.push(arg);
    const next = positional[index + 1];
    if (!arg.includes('=') && next !== undefined && takesNextArg(name, next)) {
      nodeArgs.push(positional[++index]);
    }
  }
  return watch ? { nodeArgs, tsconfig, watch } : { nodeArgs, tsconfig };
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

function isExplicitPath(token: string): boolean {
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

/** The first `length` bytes of `file`, or none when it cannot be read. */
function readHead(file: string, length: number): Buffer {
  const head = Buffer.alloc(length);
  try {
    const fd = openSync(file, 'r');
    try {
      return head.subarray(0, readSync(fd, head, 0, length, 0));
    } finally {
      closeSync(fd);
    }
  } catch {
    return head.subarray(0, 0);
  }
}

function hasVpxShebang(file: string): boolean {
  const text = readHead(file, 256).toString('utf8');
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

/** The magic numbers of executables the kernel runs: Mach-O (thin and fat) on macOS, ELF elsewhere. */
const NATIVE_MAGIC =
  process.platform === 'darwin'
    ? ['feedface', 'feedfacf', 'cefaedfe', 'cffaedfe', 'cafebabe', 'bebafeca']
    : ['7f454c46'];

function isExecutableFile(file: string): boolean {
  try {
    accessSync(file, constants.X_OK);
  } catch {
    return false;
  }
  return isFile(file);
}

function isNativeExecutable(file: string): boolean {
  return isExecutableFile(file) && NATIVE_MAGIC.includes(readHead(file, 4).toString('hex'));
}

/**
 * Whether exec can run `file` itself: an executable native binary, or an executable
 * script whose `#!` interpreter is one. A failed `process.execve()` aborts most supported
 * Node.js versions instead of throwing, so the project-local bin runs anything else
 * through `/bin/sh`, which reports a file it cannot run as a terminal does. The
 * interpreter keeps a trailing `\r`, as the kernel reads it.
 */
export function canExecDirectly(file: string): boolean {
  if (!isExecutableFile(file)) {
    return false;
  }
  const head = readHead(file, 256);
  if (head.subarray(0, 2).toString('latin1') !== '#!') {
    return isNativeExecutable(file);
  }
  const line = head.subarray(2).toString('utf8').split('\n', 1)[0];
  const interpreter = line.replace(/^[ \t]+/, '').split(/[ \t]/, 1)[0];
  return path.isAbsolute(interpreter) && isNativeExecutable(interpreter);
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

/** Every `node_modules/.bin` path from `cwd` up to the root, nearest first, whether or not it exists. */
export function nodeModulesBinPaths(cwd: string): string[] {
  const paths: string[] = [];
  for (let dir = cwd; ; dir = path.dirname(dir)) {
    paths.push(path.join(dir, 'node_modules', '.bin'));
    if (path.dirname(dir) === dir) {
      return paths;
    }
  }
}

/** The nearest `node_modules/.bin/<name>` from `cwd` up (`.cmd` on Windows), like the global CLI's `find_local_binary`. */
function findLocalBin(name: string, cwd: string): { file: string; stats: Stats } | undefined {
  const fileName = process.platform === 'win32' ? `${name}.cmd` : name;
  for (const dir of nodeModulesBinPaths(cwd)) {
    const file = path.join(dir, fileName);
    const stats = statSync(file, { throwIfNoEntry: false });
    if (stats?.isFile()) {
      return { file, stats };
    }
  }
  return undefined;
}

/** An executable mode bit, as the global CLI checks; always true on Windows. */
function isExecutable(stats: Stats): boolean {
  return process.platform === 'win32' || (stats.mode & 0o111) !== 0;
}

export interface ProgramError {
  error: string;
  note?: string;
}

/**
 * Package mode runs an explicit path as a program. An existing file must be executable,
 * and a directory needs a package.json for `vp dlx`, so either fails here with a clear
 * error instead of in `vp dlx`. Mirrors `check_explicit_program` in the global CLI.
 */
export function checkExplicitProgram(token: string, cwd: string): ProgramError | undefined {
  if (!isExplicitPath(token)) {
    return undefined;
  }
  const file = path.resolve(cwd, token);
  const stats = statSync(file, { throwIfNoEntry: false });
  if (stats?.isDirectory() && !isFile(path.join(file, 'package.json'))) {
    return { error: `'${token}' is a directory without a package.json` };
  }
  if (stats?.isFile() && !isExecutable(stats)) {
    return {
      error: `'${token}' is not executable`,
      note: `Make it executable with \`chmod +x ${token}\`.`,
    };
  }
  return undefined;
}

export type PackageInvocation =
  | { command: 'run'; file: string; args: string[] }
  | { command: 'dlx'; args: string[] }
  | ProgramError;

/**
 * How the project-local bin runs `vpx <pkg> [args...]`. The nearest binary in
 * `node_modules/.bin`, or an existing executable file, runs directly, as the global CLI
 * execs the path it finds. A versioned package, `--package`, `--shell-mode`, or a local
 * package directory runs with this package's `vp dlx`. Global packages and `PATH` are
 * not searched, so a binary that is not installed is an error rather than a download.
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
    const problem = checkExplicitProgram(spec, cwd);
    if (problem) {
      return problem;
    }
    const file = path.resolve(cwd, spec);
    return isFile(file)
      ? { command: 'run', file, args: rest }
      : { command: 'dlx', args: [...silent, spec, ...rest] };
  }
  const name = commandName(spec);
  const bin = findLocalBin(name, cwd);
  if (bin) {
    if (!isExecutable(bin.stats)) {
      const relative = path.relative(cwd, bin.file);
      return {
        error: `'${relative}' is not executable`,
        note: `Reinstall dependencies with \`vp install\`, or make it executable with \`chmod +x ${relative}\`.`,
      };
    }
    return { command: 'run', file: bin.file, args: rest };
  }
  return {
    error: `'${name}' is not installed in node_modules`,
    note: `Add it to devDependencies, or run \`vp dlx ${spec}\` to download and run it.`,
  };
}
