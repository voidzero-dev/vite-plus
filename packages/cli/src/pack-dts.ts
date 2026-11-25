/**
 * Declaration generator selection for `vp pack`.
 *
 * When a build needs declarations and the declaration plugin cannot load
 * `typescript`, select the `tsrs` generator with the `tsrs` package that
 * `vite-plus` ships for `vp lint` and `vp check`. Upstream reports "TypeScript
 * is not installed" in that case, so builds that work today are unchanged.
 * Every other case keeps the upstream selection. See
 * `rfcs/tsdown-native-declarations.md`.
 */

import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { pathToFileURL } from 'node:url';

import type { ParseError } from 'jsonc-parser';
import { parse as parseJsonc } from 'jsonc-parser';

import { resolveBundled } from './utils/constants.ts';
import type { JsonValue } from './utils/json-value.ts';
import { isJsonObject, isObjectValue, parseJsonObject } from './utils/json-value.ts';

/**
 * The declaration plugin prints this notice the first time it runs `tsrs`.
 * Under `failOnWarn` it fails a build that emitted correctly, so it is
 * suppressed when Vite+ selects `tsrs`. An info line replaces it.
 */
export const TSRS_EXPERIMENTAL_WARNING = /^tsrs support is highly experimental\b/;

export type SuppressWarnings =
  | string
  | RegExp
  | Array<string | RegExp>
  | ((message: string) => boolean);

/**
 * The declaration plugin options that the selection reads or sets. The bundled
 * plugin's own types describe the full set.
 */
export interface DtsGeneratorOptions {
  generator?: unknown;
  cwd?: string;
  tsconfig?: string | boolean;
  compilerOptions?: { isolatedDeclarations?: unknown };
  tsconfigRaw?: object;
  customLanguages?: ReadonlyArray<unknown>;
  tsc?: { vue?: unknown };
  /** Deprecated alias of `tsc.vue`. */
  vue?: unknown;
  tsrs?: { moduleUrl?: string };
}

/** The resolved tsdown config fields that the selection reads. */
export interface DtsGeneratorConfig<T extends DtsGeneratorOptions> {
  cwd: string;
  tsconfig: false | string;
  dts: false | T;
}

export interface DtsGeneratorEnvironment {
  /** Returns the path of `typescript` as the declaration plugin resolves it, if it resolves. */
  resolveTypeScript(): string | undefined;
  /** Returns the `file://` URL of the `tsrs` package that `vite-plus` ships. */
  tsrsModuleUrl(): string;
}

export type DtsGeneratorSelection<T extends DtsGeneratorOptions> =
  /** Declarations are off, or upstream keeps its selection. */
  | { kind: 'unchanged'; dts: false | T; typescriptPath?: string }
  /** The user selected `tsrs`. Only a missing `moduleUrl` was filled. */
  | { kind: 'explicit-tsrs'; dts: T }
  /** Vite+ selected `tsrs` because `typescript` did not resolve. */
  | { kind: 'tsrs'; dts: T };

/**
 * The environment of the installed `vite-plus`. The declaration plugin is
 * bundled into core's `vite/pack` and resolves `typescript` relative to
 * itself, so the lookup starts there and not at the project root.
 */
export function createDtsGeneratorEnvironment(): DtsGeneratorEnvironment {
  return {
    resolveTypeScript() {
      const require = createRequire(import.meta.resolve('vite/pack'));
      for (const specifier of ['typescript/package.json', 'typescript']) {
        try {
          return require.resolve(specifier);
        } catch {
          // Try the next specifier. TypeScript 7 may not export `package.json`.
        }
      }
      return undefined;
    },
    tsrsModuleUrl() {
      return pathToFileURL(resolveBundled('tsrs')).href;
    },
  };
}

/** The version of the `tsrs` package that `vite-plus` ships. */
export function bundledTsrsVersion(): string {
  const file = resolveBundled('tsrs/package.json');
  const { version } = parseJsonObject(fs.readFileSync(file, 'utf8'), file);
  if (typeof version !== 'string') {
    throw new TypeError(`Expected a version in ${file}`);
  }
  return version;
}

/** Whether a `pack.dts` value sets `generator` itself. */
export function hasExplicitDtsGenerator(
  dts: boolean | string | DtsGeneratorOptions | undefined,
): boolean {
  return typeof dts === 'object' && dts.generator !== undefined;
}

/** Adds {@link TSRS_EXPERIMENTAL_WARNING} to a `suppressWarnings` value and keeps its entries. */
export function withTsrsWarningSuppressed(
  suppressWarnings: SuppressWarnings | undefined,
): SuppressWarnings {
  if (suppressWarnings === undefined) {
    return TSRS_EXPERIMENTAL_WARNING;
  }
  if (typeof suppressWarnings === 'function') {
    return (message) => TSRS_EXPERIMENTAL_WARNING.test(message) || suppressWarnings(message);
  }
  return [
    ...(Array.isArray(suppressWarnings) ? suppressWarnings : [suppressWarnings]),
    TSRS_EXPERIMENTAL_WARNING,
  ];
}

function withTsrsModuleUrl<T extends DtsGeneratorOptions>(dts: T, env: DtsGeneratorEnvironment): T {
  if (dts.tsrs?.moduleUrl !== undefined) {
    return dts;
  }
  return { ...dts, tsrs: { ...dts.tsrs, moduleUrl: env.tsrsModuleUrl() } };
}

/**
 * Selects the declaration generator for one resolved tsdown config.
 *
 * Throws when Vite+ would select `tsrs` but the config sets compiler options
 * that native generators do not apply to the program.
 */
export function selectDtsGenerator<T extends DtsGeneratorOptions>(
  config: DtsGeneratorConfig<T>,
  env: DtsGeneratorEnvironment,
): DtsGeneratorSelection<T> {
  const { dts } = config;
  if (dts === false) {
    return { kind: 'unchanged', dts };
  }
  if (dts.generator === 'tsrs') {
    return { kind: 'explicit-tsrs', dts: withTsrsModuleUrl(dts, env) };
  }
  if (dts.generator !== undefined || usesCustomLanguages(dts)) {
    return { kind: 'unchanged', dts };
  }

  const typescriptPath = env.resolveTypeScript();
  if (typescriptPath !== undefined) {
    return { kind: 'unchanged', dts, typescriptPath };
  }

  // Native generators need a tsconfig file. Without one, keep the upstream error.
  const tsconfigPath = resolveDtsTsconfig(config, dts);
  if (tsconfigPath === undefined) {
    return { kind: 'unchanged', dts };
  }

  // Upstream selects Oxc for isolated declarations, which works without TypeScript.
  const isolatedDeclarations =
    dts.compilerOptions?.isolatedDeclarations ?? readIsolatedDeclarations(tsconfigPath);
  // `undefined` means that the tsconfig chain could not be read. Upstream reports that error.
  if (isolatedDeclarations !== false) {
    return { kind: 'unchanged', dts };
  }

  const overrides = (['compilerOptions', 'tsconfigRaw'] as const).filter(
    (key) => Object.keys(dts[key] ?? {}).length > 0,
  );
  if (overrides.length > 0) {
    const names = overrides.map((key) => `\`dts.${key}\``).join(' and ');
    throw new Error(
      `${names} cannot be used without \`typescript\`. ` +
        'The bundled tsrs declaration generator reads compiler options only from the tsconfig file. ' +
        `Move these options into ${tsconfigPath}, or install \`typescript\`.`,
    );
  }

  return { kind: 'tsrs', dts: withTsrsModuleUrl({ ...dts, generator: 'tsrs' }, env) };
}

function usesCustomLanguages(dts: DtsGeneratorOptions): boolean {
  return !!dts.tsc?.vue || !!dts.vue || (dts.customLanguages?.length ?? 0) > 0;
}

/**
 * Returns the tsconfig file that the declaration plugin loads. tsdown passes
 * its resolved `tsconfig` to the plugin, and `dts.tsconfig` overrides it.
 */
function resolveDtsTsconfig(
  config: Pick<DtsGeneratorConfig<DtsGeneratorOptions>, 'cwd' | 'tsconfig'>,
  dts: DtsGeneratorOptions,
): string | undefined {
  const cwd = dts.cwd ?? config.cwd;
  const tsconfig = dts.tsconfig ?? config.tsconfig;
  if (tsconfig === false) {
    return undefined;
  }
  if (typeof tsconfig === 'string') {
    const file = path.resolve(cwd, tsconfig);
    return fs.existsSync(file) ? file : undefined;
  }
  return findTsconfig(cwd);
}

function findTsconfig(dir: string): string | undefined {
  let current = path.resolve(dir);
  while (true) {
    const file = path.join(current, 'tsconfig.json');
    if (fs.existsSync(file)) {
      return file;
    }
    const parent = path.dirname(current);
    if (parent === current) {
      return undefined;
    }
    current = parent;
  }
}

/**
 * Reads the effective `compilerOptions.isolatedDeclarations` of a tsconfig,
 * including `extends`. Returns `undefined` when a file in the chain cannot be
 * read or resolved, or when the chain has a cycle.
 */
export function readIsolatedDeclarations(tsconfigPath: string): boolean | undefined {
  const value = readOption(tsconfigPath, new Set());
  return value === UNREADABLE ? undefined : value === true;
}

const UNREADABLE = Symbol('unreadable');

type OptionValue = boolean | undefined | typeof UNREADABLE;

/** `ancestors` holds the files on the current `extends` path, so a shared base is not a cycle. */
function readOption(file: string, ancestors: Set<string>): OptionValue {
  if (ancestors.has(file)) {
    return UNREADABLE;
  }
  ancestors.add(file);
  const value = readOwnOrInheritedOption(file, ancestors);
  ancestors.delete(file);
  return value;
}

function readOwnOrInheritedOption(file: string, ancestors: Set<string>): OptionValue {
  const errors: Array<ParseError> = [];
  let tsconfig: unknown;
  try {
    tsconfig = parseJsonc(fs.readFileSync(file, 'utf8'), errors, { allowTrailingComma: true });
  } catch {
    return UNREADABLE;
  }
  if (errors.length > 0 || !isJsonObject(tsconfig)) {
    return UNREADABLE;
  }

  const { compilerOptions } = tsconfig;
  if (isObjectValue(compilerOptions)) {
    const value = compilerOptions.isolatedDeclarations;
    // `null` resets an inherited value to the default, which is `false`.
    if (typeof value === 'boolean' || value === null) {
      return value === true;
    }
  }

  const extendsValue = tsconfig.extends;
  const parents: Array<JsonValue> =
    typeof extendsValue === 'string'
      ? [extendsValue]
      : Array.isArray(extendsValue)
        ? extendsValue
        : [];
  // Later entries in an `extends` array override earlier ones.
  for (const specifier of parents.toReversed()) {
    const parent = typeof specifier === 'string' ? resolveExtends(specifier, file) : undefined;
    if (parent === undefined) {
      return UNREADABLE;
    }
    const value = readOption(parent, ancestors);
    if (value !== undefined) {
      return value;
    }
  }
  return undefined;
}

function resolveExtends(specifier: string, fromFile: string): string | undefined {
  const dir = path.dirname(fromFile);
  if (path.isAbsolute(specifier) || specifier.startsWith('.')) {
    const file = path.resolve(dir, specifier);
    for (const candidate of [file, `${file}.json`]) {
      if (fs.existsSync(candidate) && fs.statSync(candidate).isFile()) {
        return candidate;
      }
    }
    return undefined;
  }
  const require = createRequire(fromFile);
  for (const candidate of [specifier, `${specifier}.json`, `${specifier}/tsconfig.json`]) {
    try {
      return require.resolve(candidate);
    } catch {
      // Try the next form. TypeScript resolves these forms for package `extends`.
    }
  }
  return undefined;
}
