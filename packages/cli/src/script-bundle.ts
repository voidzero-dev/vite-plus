/**
 * The opt-in bundle cache for `vpx <script>` (`VPX_BUNDLE_CACHE=1`); see
 * `rfcs/vpx-bundle-cache.md`.
 *
 * A first run loads the script one module at a time, as without the cache, and records
 * how each import resolved and in which format each module loaded. At exit it starts
 * `script-bundle-build.js`, which bundles the script's ES modules with Rolldown into the
 * cache. Later runs check that every input is unchanged, then run the bundle, with the
 * native loader loaded only for a module the bundle does not contain.
 *
 * The preload (`script-preload.ts`) reads this module before it decides whether to load
 * the native binding, so it uses only Node.js built-ins.
 */

import {
  mkdirSync,
  readFileSync,
  realpathSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
import { isBuiltin } from 'node:module';
import path from 'node:path';
import { pathToFileURL, fileURLToPath } from 'node:url';

import { SCRIPT_TSCONFIG_ENV, TYPESCRIPT_DEPENDENCY } from './script-hooks.ts';

/** Bump when the manifest, the trace, or the bundle layout changes. */
export const BUNDLE_CACHE_VERSION = 1;

const DEBUG_PREFIX = 'vpx bundle cache:';

export function debugEnabled(env: NodeJS.ProcessEnv = process.env): boolean {
  return env.VPX_BUNDLE_CACHE_DEBUG === '1';
}

/** One diagnostic line on stderr under `VPX_BUNDLE_CACHE_DEBUG=1`. */
export function debug(message: string, env: NodeJS.ProcessEnv = process.env): void {
  if (debugEnabled(env)) {
    process.stderr.write(`${DEBUG_PREFIX} ${message}\n`);
  }
}

/**
 * A file's identity and content stamp, or `''` when it does not exist. The change time
 * catches an edit that keeps the size and restores the modification time, which
 * `utimes()` can do and which a change time cannot undo; the inode catches a file
 * replaced by a rename. Number stats keep microsecond times and take about half the
 * time of bigint stats in a fresh process.
 */
export function stamp(file: string): string {
  const stats = statSync(file, { throwIfNoEntry: false });
  return stats ? `${stats.ino}:${stats.size}:${stats.mtimeMs}:${stats.ctimeMs}` : '';
}

/** A file or directory, and its `stamp()` when the cache was built. */
export type Input = [path: string, stamp: string];

export interface BundleManifest {
  version: number;
  /** `bundleKey()` of the run that recorded the trace. */
  key: string;
  /** When set, the script runs without a bundle until an input changes. */
  disabled?: string;
  /** The output directory, relative to the entry's cache directory. */
  out?: string;
  /** The entry chunk, relative to `out`. */
  entry?: string;
  /** Every chunk, relative to `out`. */
  chunks?: string[];
  /** The `file:` URLs of the source modules in the bundle. */
  modules?: string[];
  inputs: Input[];
}

/** What one run of the script recorded for the builder. */
export interface BundleTrace {
  version: number;
  key: string;
  /** The entry module's `file:` URL, as the main entry resolves. */
  entry: string;
  cwd: string;
  /** Distinct export condition lists; resolves refer to them by index. */
  conditions: string[][];
  /**
   * `[parentURL, specifier, kind, conditions index, resolved URL, resolved format]`.
   * The kind is `import` for an ES module request (static or dynamic) and `require`
   * for a CommonJS one. The entry has no parent.
   */
  resolves: [string | null, string, 'import' | 'require', number, string, string | null][];
  /** `[URL, format]` as each module finally loaded. */
  loads: [string, string | null][];
}

/** The paths of one entry's cache. */
export interface BundleCacheContext {
  /** The script, as Node.js resolves the main entry. */
  entryPath: string;
  entryURL: string;
  cwd: string;
  key: string;
  /** `<project>/node_modules/.vite-plus/vpx-bundle`. */
  root: string;
  /** This entry's directory in `root`. */
  dir: string;
}

/** Node.js options that load code or hooks the bundle would bypass. */
const LOADER_OPTION = /^--(?:require|import|loader|experimental-loader)(?:=|$)|^-r$/;
const LOADER_OPTION_IN_NODE_OPTIONS =
  /(?:^|\s)(?:--(?:require|import|loader|experimental-loader)|-r)(?:[=\s]|$)/;
/**
 * Node.js options under which the script, coverage, or symlinks behave differently, and
 * a config file, which can add preloads and conditions that `execArgv` does not show.
 */
const UNSUPPORTED_OPTION =
  /^--(?:test|watch|watch-path|check|eval|print|interactive|experimental-test-coverage|preserve-symlinks|preserve-symlinks-main|experimental-config-file|experimental-default-config-file)(?:=|$)|^-(?:e|p|c|i)$/;

/** The real paths of the preload's own `--require` and `--import` entries. */
export interface LoaderEntries {
  preload: string;
  register: string;
}

/**
 * Whether an option value names `file`. The CLIs pass the preload as a path and the
 * `--import` entry as a `file:` URL, either possibly through a symlink.
 */
function namesFile(value: string | undefined, file: string): boolean {
  if (value === undefined) {
    return false;
  }
  try {
    const named = value.startsWith('file:') ? fileURLToPath(value) : path.resolve(value);
    return named === file || realpathSync(named) === file;
  } catch {
    return false;
  }
}

/** Why the cache cannot serve this process, or `undefined` when it can. */
export function unsupportedReason(
  env: NodeJS.ProcessEnv,
  execArgv: readonly string[],
  own: LoaderEntries,
): string | undefined {
  if (env.NODE_V8_COVERAGE !== undefined) {
    return 'NODE_V8_COVERAGE is set';
  }
  if (env.NODE_PRESERVE_SYMLINKS === '1') {
    return 'NODE_PRESERVE_SYMLINKS is set';
  }
  if (LOADER_OPTION_IN_NODE_OPTIONS.test(env.NODE_OPTIONS ?? '')) {
    return 'NODE_OPTIONS loads other modules or hooks';
  }
  for (let i = 0; i < execArgv.length; i++) {
    const arg = execArgv[i];
    const next = execArgv[i + 1];
    if (
      (arg === '--require' && namesFile(next, own.preload)) ||
      (arg === '--import' && namesFile(next, own.register))
    ) {
      i++;
      continue;
    }
    if (LOADER_OPTION.test(arg)) {
      return `${arg.split('=', 1)[0]} loads other modules or hooks`;
    }
    if (UNSUPPORTED_OPTION.test(arg)) {
      return `${arg.split('=', 1)[0]} is not supported`;
    }
  }
  return undefined;
}

/** The project directory's `node_modules/.vite-plus/vpx-bundle`, nearest the entry. */
export function findCacheRoot(entryPath: string): string | undefined {
  for (let dir = path.dirname(entryPath); ;) {
    const modules = path.join(dir, 'node_modules');
    if (statSync(modules, { throwIfNoEntry: false })?.isDirectory()) {
      return path.join(modules, '.vite-plus', 'vpx-bundle');
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      return undefined;
    }
    dir = parent;
  }
}

/** A 53-bit string hash (cyrb53) for directory names; the manifest keeps the full key. */
export function hash(value: string): string {
  let h1 = 0xdeadbeef;
  let h2 = 0x41c6ce57;
  for (let i = 0; i < value.length; i++) {
    const ch = value.charCodeAt(i);
    h1 = Math.imul(h1 ^ ch, 2654435761);
    h2 = Math.imul(h2 ^ ch, 1597334677);
  }
  h1 = Math.imul(h1 ^ (h1 >>> 16), 2246822507) ^ Math.imul(h2 ^ (h2 >>> 13), 3266489909);
  h2 = Math.imul(h2 ^ (h2 >>> 16), 2246822507) ^ Math.imul(h1 ^ (h1 >>> 13), 3266489909);
  return (4294967296 * (2097151 & h2) + (h1 >>> 0)).toString(36);
}

/**
 * Everything outside the files that decides how the script loads: the runtime, the
 * entry, the working directory (tsconfig discovery starts there), the Node.js options
 * (export conditions), and the loader's configuration variables.
 */
export function bundleKey(
  entryPath: string,
  cwd: string,
  env: NodeJS.ProcessEnv,
  execArgv: readonly string[],
): string {
  return JSON.stringify({
    v: BUNDLE_CACHE_VERSION,
    node: process.version,
    platform: process.platform,
    arch: process.arch,
    entry: entryPath,
    cwd,
    execArgv,
    nodeOptions: env.NODE_OPTIONS ?? null,
    env: SCRIPT_TSCONFIG_ENV.map((name) => env[name] ?? null),
  });
}

/** The cache paths for this process's entry, or `undefined` without a project. */
export function bundleCacheContext(
  argv1: string | undefined,
  cwd: string,
  env: NodeJS.ProcessEnv,
  execArgv: readonly string[],
): BundleCacheContext | undefined {
  if (argv1 === undefined) {
    return undefined;
  }
  let entryPath;
  try {
    // The main entry resolves to its real path (no `--preserve-symlinks-main` here).
    entryPath = realpathSync(argv1);
  } catch {
    return undefined;
  }
  const root = findCacheRoot(entryPath);
  if (root === undefined) {
    return undefined;
  }
  const key = bundleKey(entryPath, cwd, env, execArgv);
  return {
    entryPath,
    entryURL: pathToFileURL(entryPath).href,
    cwd,
    key,
    root,
    dir: path.join(root, `${path.basename(entryPath).replace(/[^\w.-]/g, '_')}-${hash(key)}`),
  };
}

export const MANIFEST = 'manifest.json';
export const TRACE = 'trace.json';
export const EXCLUDED = 'excluded.json';
const LOCK = 'build.lock';
/** A lock older than this belongs to a builder that died. */
const LOCK_STALE_MS = 60_000;

export function readJson(file: string): unknown {
  try {
    return JSON.parse(readFileSync(file, 'utf8')) as unknown;
  } catch {
    return undefined;
  }
}

/** Write through a temporary sibling, so a reader never sees a partial file. */
export function writeJsonAtomic(file: string, value: unknown): void {
  mkdirSync(path.dirname(file), { recursive: true });
  const temp = `${file}.${process.pid}.tmp`;
  writeFileSync(temp, JSON.stringify(value));
  renameSync(temp, file);
}

/** The first input that changed since the build, or `undefined` when none did. */
export function changedInput(inputs: readonly Input[]): string | undefined {
  for (const [file, expected] of inputs) {
    if (stamp(file) !== expected) {
      return file;
    }
  }
  return undefined;
}

export type ManifestState =
  | { kind: 'hit'; manifest: BundleManifest }
  | { kind: 'disabled'; reason: string }
  | { kind: 'miss'; reason: string };

/** Whether this entry has a usable bundle, a still-valid reason not to, or neither. */
export function checkManifest(context: BundleCacheContext): ManifestState {
  const manifest = readJson(path.join(context.dir, MANIFEST)) as BundleManifest | undefined;
  if (manifest === undefined) {
    return { kind: 'miss', reason: 'no bundle' };
  }
  if (manifest.version !== BUNDLE_CACHE_VERSION || manifest.key !== context.key) {
    return { kind: 'miss', reason: 'another configuration' };
  }
  const changed = changedInput(manifest.inputs);
  if (changed !== undefined) {
    return { kind: 'miss', reason: `${changed} changed` };
  }
  if (manifest.disabled !== undefined) {
    return { kind: 'disabled', reason: manifest.disabled };
  }
  if (manifest.out === undefined || manifest.entry === undefined) {
    return { kind: 'miss', reason: 'incomplete manifest' };
  }
  // A missing chunk would fail the run, not just the cache.
  const out = path.join(context.dir, manifest.out);
  for (const chunk of manifest.chunks ?? [manifest.entry]) {
    if (statSync(path.join(out, chunk), { throwIfNoEntry: false }) === undefined) {
      return { kind: 'miss', reason: `${path.join(out, chunk)} is missing` };
    }
  }
  return { kind: 'hit', manifest };
}

/** Take the build lock, or return `false` while another builder holds it. */
export function acquireBuildLock(dir: string): boolean {
  const lock = path.join(dir, LOCK);
  mkdirSync(dir, { recursive: true });
  try {
    mkdirSync(lock);
    return true;
  } catch {
    const age = Date.now() - (statSync(lock, { throwIfNoEntry: false })?.mtimeMs ?? 0);
    if (age < LOCK_STALE_MS) {
      return false;
    }
    rmSync(lock, { recursive: true, force: true });
    try {
      mkdirSync(lock);
      return true;
    } catch {
      return false;
    }
  }
}

export function releaseBuildLock(dir: string): void {
  rmSync(path.join(dir, LOCK), { recursive: true, force: true });
}

/**
 * Remember a module the bundle must not contain, and drop the bundle, so the next run
 * records the script again and the builder leaves the module out.
 */
export function excludeModule(dir: string, url: string): void {
  const file = path.join(dir, EXCLUDED);
  const excluded = new Set((readJson(file) as string[] | undefined) ?? []);
  excluded.add(url);
  writeJsonAtomic(file, [...excluded]);
  rmSync(path.join(dir, MANIFEST), { force: true });
}

/**
 * Whether Node.js' own resolution and load answer a `file:` URL exactly as the loader
 * does: a JavaScript file in `node_modules`. The loader leaves those to Node.js, while
 * it transforms TypeScript there, turns JSON into a module, and transforms every file
 * outside `node_modules`.
 */
export function isPlainDependency(url: string): boolean {
  return (
    url.startsWith('file:') &&
    url.includes('/node_modules/') &&
    !TYPESCRIPT_DEPENDENCY.test(url) &&
    !url.endsWith('.json')
  );
}

/** Whether a specifier names a Node.js built-in, with or without `node:`. */
export function isBuiltinSpecifier(specifier: string): boolean {
  return specifier.startsWith('node:') || isBuiltin(specifier);
}

/**
 * Whether Node.js' own resolution answers a relative import in a dependency exactly as
 * the loader does: the specifier names a JavaScript file that exists. The loader tries
 * the named file first for an importer in `node_modules`, and leaves its format to
 * Node.js.
 */
export function isPlainDependencyImport(specifier: string, parentURL: string): boolean {
  if (
    !(specifier.startsWith('./') || specifier.startsWith('../')) ||
    !isPlainDependency(parentURL) ||
    /[?#%\\]/.test(specifier)
  ) {
    return false;
  }
  const url = new URL(specifier, parentURL);
  return (
    isPlainDependency(url.href) &&
    /\.[cm]?js$/.test(url.pathname) &&
    statSync(url, { throwIfNoEntry: false })?.isFile() === true
  );
}

/**
 * The bundle prefixes a specifier that only the run knows (a computed `import()`, the
 * argument of `import.meta.resolve()`) with `vpx-from:<index>:`, where `<index>` is the
 * importing module's position in `BundleManifest.modules`, so the hooks resolve it from
 * that module's URL.
 */
export const RESOLVE_FROM_PREFIX = 'vpx-from:';

/** The module index and the specifier of a `RESOLVE_FROM_PREFIX` specifier. */
export function parseResolveFrom(
  specifier: string,
): [index: number, specifier: string] | undefined {
  if (!specifier.startsWith(RESOLVE_FROM_PREFIX)) {
    return undefined;
  }
  const end = specifier.indexOf(':', RESOLVE_FROM_PREFIX.length);
  const index = Number(specifier.slice(RESOLVE_FROM_PREFIX.length, end));
  return end < 0 || !Number.isInteger(index) ? undefined : [index, specifier.slice(end + 1)];
}
