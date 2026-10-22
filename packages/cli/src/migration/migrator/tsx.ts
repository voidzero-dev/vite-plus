import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';
import { parse as parseJsonc } from 'jsonc-parser';

import { type LiteralCommandToken, parseLiteralCommands } from '../../../binding/index.js';
import { type WorkspacePackage } from '../../types/index.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import { readNearestPackageJson } from '../../utils/package.ts';
import { displayRelative } from '../../utils/path.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import {
  NODE_OPTIONS_WITH_VALUE,
  SCRIPT_EXTENSIONS,
  WATCH_OPTIONS_WITH_VALUE,
} from '../../vpx-script.ts';
import { sourceTreeMatches } from '../migrator.ts';
import { type MigrationReport } from '../report.ts';
import {
  type DependencyBag,
  getCommandStartIndex,
  isPlainRecord,
  LINT_STAGED_JSON_CONFIG_FILES,
  readPackageJsonIfExists,
  RETENTION_SCAN_SKIP_DIRS,
  SHELL_STATE_COMMANDS,
  warnMigration,
} from './shared.ts';

// tsx in command position: `tsx a.ts`, `npx tsx`, `--import tsx`, `--import=tsx`,
// `-r tsx/cjs`, `node_modules\.bin\tsx.cmd`, or inside a quoted command, in any case, as
// Windows finds `TSX`. Not `.tsx` files or packages such as `tsx-foo`.
const TSX_COMMAND =
  /(?:^|[\s;&|("'=/\\`])tsx(?:\.(?:cmd|exe|ps1))?(?:\/(?:esm|cjs))?(?=$|[\s;&|)"'`])/i;

// Node.js options that load tsx's hooks, as in `node --import tsx ./a.ts`.
const LOADER_OPTIONS = new Set([
  '--import',
  '--loader',
  '--experimental-loader',
  '--require',
  '-r',
]);
const TSX_LOADER = /^tsx(?:\/(?:esm|cjs))?$/;
// Tool settings that name modules to preload, as in Mocha's `"import": "tsx"`.
const LOADER_SETTINGS = new Set(['import', 'loader', 'require']);

const KEEP_EVAL = '`vpx` does not run TypeScript passed to --eval or --print';
const KEEP_CHECK = '`vpx` does not transform TypeScript for -c or --check';
const KEEP_REPL = '`vpx` does not start a TypeScript REPL';
const KEEP_TEST_GLOBS =
  '`--test` without files finds `.ts` tests with tsx globs; list the test files for `vpx --test`';
const KEEP_WATCH_PATH = '`--watch-path` has no `vpx` equivalent; use `--include`';
const KEEP_SHELL =
  'the command uses shell syntax that vp migrate does not rewrite, such as pipes, variables, or subshells';
const KEEP_OTHER = 'vp migrate does not rewrite this use of tsx';
const KEEP_TSX_TSCONFIG =
  '`TSX_TSCONFIG_PATH` selects the tsconfig for tsx only; pass it to `vpx` as `--tsconfig`';
const KEEP_NODE_OPTIONS =
  '`NODE_OPTIONS` loads tsx, whose hooks would compete with those of `vpx`; remove it first';
const KEEP_WATCH_TEST = '`vpx --watch` does not run `--test`';

// Options that `vpx` does not run as tsx does. `-c` is Node.js' `--check`, but `vpx -c`
// is `--shell-mode`.
const KEEP_OPTIONS = new Map([
  ['-e', KEEP_EVAL],
  ['--eval', KEEP_EVAL],
  ['-p', KEEP_EVAL],
  ['--print', KEEP_EVAL],
  ['-pe', KEEP_EVAL],
  ['-c', KEEP_CHECK],
  ['--check', KEEP_CHECK],
  ['-i', KEEP_REPL],
  ['--interactive', KEEP_REPL],
  ['--watch-path', KEEP_WATCH_PATH],
]);

// A tsx shebang `vpx` can take over: `#!/usr/bin/env tsx`, `#!/usr/bin/env -S tsx --tsconfig x.json`.
const TSX_ENV_SHEBANG = /^(#!\s*\S*\benv\s+(?:-S\s+)?)(tsx(?=[ \t\r\n]|$)[^\r\n]*)/;
// Stands for the file that the kernel appends to a shebang's command.
const SHEBANG_ENTRY = './shebang.ts';
// Any first line that runs tsx, including `#!/usr/local/bin/tsx`.
const TSX_SHEBANG = /^#![^\n]*[\s/]tsx(?=\s|$)/;
// tsx loaded into Node.js by a flag, as in `--import=tsx` or `-r tsx/cjs`, or by Mocha's
// dashless `node-option` value `import=tsx`.
const TSX_LOADER_FLAG =
  /(?:(?:--import|--loader|--experimental-loader|--require|-r)[= \t]+|(?<![\w-])(?:import|loader|experimental-loader|require)=)['"]?tsx(?:\/(?:esm|cjs))?(?![\w./-])/;
// In code: an import of tsx or a subpath such as `tsx/esm/api`, also after a comment as in
// `import(/* @vite-ignore */ 'tsx/esm/api')`, a string that starts a tsx command, tsx as
// the program of a process call, as in `spawn('tsx')` or `Bun.spawn(['tsx', ...])`, or a
// loader flag, as in `nodeArguments: ['--import=tsx']` or `['--import', 'tsx']`.
const TSX_CODE_REFERENCE = new RegExp(
  [
    /(?:\bfrom|\bimport|\bimport\.meta\.resolve|\brequire(?:\.resolve)?)\s*\(?\s*(?:\/\*[\s\S]*?\*\/\s*)*['"`]tsx(?:\/[^'"`]*)?['"`]/
      .source,
    /['"`](?:npx |pnpm (?:exec )?|yarn |bunx )?tsx\s/.source,
    /\(\s*\[?\s*['"`]tsx['"`]\s*[,)\]]/.source,
    TSX_LOADER_FLAG.source,
    /['"`](?:--import|--loader|--experimental-loader|--require|-r)['"`]\s*,\s*['"`]tsx(?:\/(?:esm|cjs))?['"`]/
      .source,
  ].join('|'),
);
// In shell scripts, CI workflows, and git hooks: tsx in command position, with or
// without arguments, or a loader flag.
const TSX_TEXT_REFERENCE = new RegExp(
  [
    /(?:^|[\s;&|("'=/\\`])tsx(?:\.(?:cmd|exe|ps1))?(?:\/(?:esm|cjs))?['"]?(?:[ \t]+\S|[ \t]*$)/
      .source,
    TSX_LOADER_FLAG.source,
  ].join('|'),
  'm',
);
// A config string that runs or loads tsx; see `jsonUsesTsx`.
const TSX_CONFIG_VALUE = new RegExp(
  [
    /(?:^|[\s;&|("'=/\\`])tsx(?:\.(?:cmd|exe|ps1))?[ \t]+\S/.source,
    TSX_LOADER_FLAG.source,
    /^tsx\//.source,
  ].join('|'),
  'i',
);
// package.json fields that name tsx without running it: dependency declarations and prose.
const IGNORED_FIELDS = new Set([
  'bundleDependencies',
  'bundledDependencies',
  'dependencies',
  'description',
  'devDependencies',
  'keywords',
  'name',
  'optionalDependencies',
  'overrides',
  'peerDependencies',
  'peerDependenciesMeta',
  'pnpm',
  'resolutions',
]);
// Script files and framework components whose code can import tsx.
const CODE_EXTENSIONS: ReadonlySet<string> = new Set([
  ...SCRIPT_EXTENSIONS,
  '.astro',
  '.svelte',
  '.vue',
]);
const REFERENCE_EXTENSIONS: ReadonlySet<string> = new Set([
  ...CODE_EXTENSIONS,
  '.json',
  '.jsonc',
  '.toml',
  '.yml',
  '.yaml',
  '.sh',
  '.bash',
  '.zsh',
  '.fish',
  '.ps1',
  '.cmd',
  '.bat',
]);
// Lockfiles name tsx without running it.
const LOCKFILES = new Set([
  'bun.lock',
  'npm-shrinkwrap.json',
  'package-lock.json',
  'pnpm-lock.yaml',
  'yarn.lock',
]);
// The scan for remaining tsx uses reads build output, which can still load tsx, but not
// caches and reports.
const USE_SCAN_SKIP_DIRS: ReadonlySet<string> = new Set([
  ...RETENTION_SCAN_SKIP_DIRS,
  '.cache',
  '.next',
  '.nuxt',
  '.svelte-kit',
  '.turbo',
  '.vite',
  'coverage',
]);

// ES module syntax at the start of a statement, including compact forms such as
// `export{}` and `export*from`, and uses of the CommonJS globals an ES module does not
// have. TypeScript's `import x = require()` and `export =` stay CommonJS.
const ESM_SYNTAX =
  /(?:^|;)\s*(?:import(?![\w$])\s*(?![\w$]+\s*=)[\w*{'"]|export(?![\w$])\s*[\w*{])|\bimport\.meta\b/m;
const COMMONJS_GLOBALS = [
  ['__dirname', /(?<![.\w$])__dirname\b/],
  ['__filename', /(?<![.\w$])__filename\b/],
  // `require('x')`, `require.resolve()`, `require.main === module`.
  ['require', /(?<![.\w$])require\s*(?:\(|\.\s*[\w$])/],
  // `exports.x`, `exports['x']`, `Object.assign(exports, ...)`; not `pkg.exports` or a key.
  ['exports', /(?<![.\w$'"`])exports(?![\w$'"`]|\s*:)/],
  // `module.exports`, `module.filename`, `module['exports']`.
  ['module', /(?<![.\w$])module\s*(?:\.\s*[\w$]|\[)/],
] as const;
// TypeScript's `export =`, which compiles to `module.exports`.
const EXPORT_ASSIGNMENT = /(?:^|;)\s*export\s*=/m;
// A relative `require()` of a JavaScript path, which tsx also resolves to TypeScript.
const RELATIVE_REQUIRE =
  /(?<![.\w$])require\s*\(\s*['"`](\.{1,2}\/[^'"`]+?)\.(js|cjs|mjs|jsx)['"`]/g;
const TYPESCRIPT_EXTENSIONS: Record<string, readonly string[]> = {
  js: ['ts', 'tsx'],
  cjs: ['cts'],
  mjs: ['mts'],
  jsx: ['tsx'],
};
// Programs that run commands given as quoted arguments, and which arguments those are,
// judged by the argument before: every one for `concurrently "tsx watch a.ts" "vp dev"`,
// only the value of `-c` for a shell. Elsewhere, a quoted string is data, as for `echo`.
const shellCommand = (previous: string) => /^-[a-z]*c$/.test(previous);
const COMMAND_LAUNCHERS = new Map<string, (previous: string) => boolean>([
  ['bash', shellCommand],
  ['concurrently', () => true],
  ['nodemon', (previous) => previous === '--exec' || previous === '-x'],
  ['sh', shellCommand],
  ['zsh', shellCommand],
]);

export interface TsxCommandRewrite {
  command: string;
  /** Why tsx stays in part of the command. */
  kept: string[];
}

interface Edit {
  start: number;
  end: number;
  text: string;
}

interface Rewrite {
  command: string;
  /** The directory the next segment runs in, or undefined when it is not known. */
  cwd?: string;
  edits: Edit[];
  kept: string[];
}

/**
 * Rewrite the tsx invocations in a command to `vpx`: `tsx a.ts` becomes `vpx a.ts`,
 * `tsx watch a.ts` becomes `vpx --watch a.ts` (`vpx watch` runs the npm package `watch`),
 * and `node --import tsx a.ts` becomes `vpx a.ts`. Package-manager wrappers such as
 * `npx tsx` go away with tsx, since `vpx` is a bin of vite-plus. A form `vpx` cannot run
 * stays as it is, with the reason in `kept`. With `cwd`, the directory the command runs
 * in, the scripts it runs are checked for syntax `vpx` runs differently.
 */
export function rewriteTsxCommand(command: string, cwd?: string): TsxCommandRewrite {
  if (!TSX_COMMAND.test(command)) {
    return { command, kept: [] };
  }
  const segments = parseLiteralCommands(command, true);
  // The parser rejects `$VAR`; cmd.exe expands `%VAR%` too.
  if (!segments || /%\w+%/.test(command)) {
    return { command, kept: [KEEP_SHELL] };
  }
  const rewrite: Rewrite = { command, cwd, edits: [], kept: [] };
  for (const tokens of segments) {
    const name = tokens[0].value;
    // After `cd` or another shell state change, scripts are no longer checked.
    if (SHELL_STATE_COMMANDS.includes(name) || name === 'chdir') {
      rewrite.cwd = undefined;
      continue;
    }
    rewriteSegment(rewrite, tokens);
  }
  let rewritten = command;
  for (const edit of rewrite.edits.toSorted((a, b) => b.start - a.start)) {
    rewritten = rewritten.slice(0, edit.start) + edit.text + rewritten.slice(edit.end);
  }
  if (!rewrite.kept.length && TSX_COMMAND.test(rewritten)) {
    rewrite.kept.push(KEEP_OTHER);
  }
  return { command: rewritten, kept: rewrite.kept };
}

function rewriteSegment(rewrite: Rewrite, tokens: LiteralCommandToken[]): void {
  const values = tokens.map(({ value }) => value);
  // Environment assignments and cross-env stay in front of the command.
  let first = 0;
  const skipAssignments = () => {
    while (/^[A-Za-z_]\w*=/.test(values[first] ?? '')) {
      first++;
    }
  };
  skipAssignments();
  // cross-env-shell runs its command, given as one quoted argument, in a shell.
  const shell = values[first] === 'cross-env-shell';
  if (values[first] === 'cross-env' || shell) {
    first++;
    skipAssignments();
  }
  if (first >= values.length) {
    return;
  }
  // tsx reads TSX_TSCONFIG_PATH, where `vpx` takes `--tsconfig`, and tsx hooks loaded
  // through NODE_OPTIONS would compete with those of `vpx`. Both reach the commands a
  // launcher runs.
  const envReason = envKeepReason(values.slice(0, first));
  const start = first + getCommandStartIndex(values.slice(first));
  const launcher = COMMAND_LAUNCHERS.get(values[start]);
  if (values[start] === 'tsx') {
    rewriteTsx(rewrite, tokens, first, start, envReason);
  } else if (values[start] === 'node') {
    rewriteNodeLoader(rewrite, tokens, first, start, envReason);
  } else if (shell) {
    for (const token of tokens.slice(start)) {
      rewriteQuotedCommand(rewrite, token, envReason);
    }
  } else if (launcher) {
    for (let index = start + 1; index < tokens.length; index++) {
      if (launcher(values[index - 1])) {
        rewriteQuotedCommand(rewrite, tokens[index], envReason);
      }
    }
  }
}

/**
 * Why environment assignments keep tsx for the command they apply to, or undefined.
 * Names match in any case, as on Windows.
 */
function envKeepReason(assignments: readonly string[]): string | undefined {
  if (assignments.some((value) => /^TSX_TSCONFIG_PATH=/i.test(value))) {
    return KEEP_TSX_TSCONFIG;
  }
  if (
    assignments.some((value) => /^NODE_OPTIONS=/i.test(value) && TSX_COMMAND.test(value.slice(13)))
  ) {
    return KEEP_NODE_OPTIONS;
  }
  return undefined;
}

/**
 * An option's name without `=value`. Node.js reads `_` as `-` in long option names, as
 * in `--watch_path`, and so does `vpx`.
 */
function optionName(arg: string): string {
  const name = arg.split('=', 1)[0];
  return name.startsWith('--') ? `--${name.slice(2).replaceAll('_', '-')}` : name;
}

/** Remove `tokens[index]` with the space before it. */
function removal(tokens: LiteralCommandToken[], index: number): Edit {
  return { start: tokens[index - 1].end, end: tokens[index].end, text: '' };
}

/**
 * Why `vpx` would fail on a script that tsx runs, judged from its source. tsx resolves
 * `require('./a.js')` to `a.ts`, and compiles ES module syntax to CommonJS in a `.cts`
 * file and in a package without `"type": "module"`. `vpx` resolves `require()` as
 * Node.js does, rejects ES module syntax in a `.cts` file, and runs the other files as ES
 * modules, so `__dirname`, `__filename`, and `require` are not defined.
 */
function scriptKeepReason(cwd: string | undefined, script: string): string | undefined {
  // A glob is not checked: its files can differ when the command runs.
  if (
    cwd === undefined ||
    /[*?[{]/.test(script) ||
    !SCRIPT_EXTENSIONS.some((extension) => script.endsWith(extension))
  ) {
    return undefined;
  }
  const file = script.startsWith('~/')
    ? path.join(os.homedir(), script.slice(2))
    : path.resolve(cwd, script);
  let source: string;
  try {
    source = fs.readFileSync(file, 'utf8');
  } catch (error) {
    // A file that an earlier command generates cannot be checked yet.
    return (error as NodeJS.ErrnoException).code === 'ENOENT'
      ? `\`${script}\` does not exist yet, so vp migrate cannot check it`
      : undefined;
  }
  for (const [, base, extension] of source.matchAll(RELATIVE_REQUIRE)) {
    const resolve = (ext: string) => path.resolve(path.dirname(file), `${base}.${ext}`);
    const typescript = fs.existsSync(resolve(extension))
      ? undefined
      : TYPESCRIPT_EXTENSIONS[extension].find((ext) => fs.existsSync(resolve(ext)));
    if (typescript) {
      return `\`${script}\` requires \`${base}.${extension}\`, which only tsx resolves to \`${base}.${typescript}\`; write the TypeScript extension`;
    }
  }
  // A comment can start a line before ES module syntax, as in `/* banner */ import ...`.
  const code = stripComments(source);
  if (!/\.(?:c?ts|tsx|jsx?)$/.test(script) || !(ESM_SYNTAX.test(source) || ESM_SYNTAX.test(code))) {
    return undefined;
  }
  if (script.endsWith('.cts')) {
    return `\`${script}\` uses import or export, which \`vpx\` does not compile to CommonJS; rename it to .mts`;
  }
  if (
    (usesCommonJsGlobals(code) || EXPORT_ASSIGNMENT.test(code)) &&
    readNearestPackageJson(path.dirname(file))?.type !== 'module'
  ) {
    return `\`${script}\` mixes import or export with CommonJS globals such as __dirname, require, or module.exports, and \`vpx\` runs it as an ES module; use import.meta.dirname, import.meta.filename, createRequire(), or export`;
  }
  return undefined;
}

/** `source` without comments; a string that only looks like one, as in `'http://x'`, stays. */
function stripComments(source: string): string {
  return source.replaceAll(
    /("(?:\\.|[^"\\\n])*"|'(?:\\.|[^'\\\n])*'|`(?:\\.|[^`\\])*`)|\/\*[\s\S]*?\*\/|\/\/[^\n]*/g,
    (_, string: string | undefined) => string ?? '',
  );
}

/**
 * Whether `code`, without comments, uses a CommonJS global it does not declare or import
 * at module level, as in `const require = createRequire(import.meta.url)` or `import
 * module from 'node:module'`. A declaration inside a function, which an unindented line
 * does not start, covers only that function.
 */
function usesCommonJsGlobals(code: string): boolean {
  return COMMONJS_GLOBALS.some(
    ([name, use]) =>
      use.test(code) &&
      !new RegExp(
        `^(?:(?:export\\s+)?(?:const|let|var|function|class)\\s+${name}\\b|import\\s+(?:\\*\\s+as\\s+)?${name}\\b)`,
        'm',
      ).test(code),
  );
}

/**
 * Why `vpx` cannot run what tsx or `node --import tsx` runs: `positionals` are the entry
 * and its arguments, or with `--test`, the test files; `preloads` are local `--require`
 * and `--import` modules, which load first.
 */
function runKeepReason(
  cwd: string | undefined,
  test: boolean,
  positionals: readonly string[],
  preloads: readonly string[],
): string | undefined {
  const [entry] = positionals;
  if (entry === undefined) {
    return test ? KEEP_TEST_GLOBS : KEEP_REPL;
  }
  // tsx finds `src/index.ts` for `src/index`; `vpx` reads a name without an extension as a package.
  if (!test && !SCRIPT_EXTENSIONS.some((extension) => entry.endsWith(extension))) {
    return `\`${entry}\` has no file extension, which \`vpx\` needs to run it as a script`;
  }
  const scripts = test ? positionals.filter((value) => !value.startsWith('-')) : [entry];
  for (const script of [...preloads, ...scripts]) {
    const reason = scriptKeepReason(cwd, script);
    if (reason) {
      return reason;
    }
  }
  return undefined;
}

function rewriteTsx(
  rewrite: Rewrite,
  tokens: LiteralCommandToken[],
  first: number,
  start: number,
  keepReason: string | undefined,
): void {
  if (keepReason) {
    rewrite.kept.push(keepReason);
    return;
  }
  let index = start + 1;
  const watch = tokens[index]?.value === 'watch';
  if (watch) {
    index++;
  }
  const scanned = scanOptions(tokens, index, true);
  const reason = runReason(rewrite.cwd, tokens, scanned, watch);
  if (reason) {
    rewrite.kept.push(reason);
    return;
  }
  rewrite.edits.push(
    { start: tokens[first].start, end: tokens[index - 1].end, text: watch ? 'vpx --watch' : 'vpx' },
    ...scanned.removals,
  );
}

function rewriteNodeLoader(
  rewrite: Rewrite,
  tokens: LiteralCommandToken[],
  first: number,
  start: number,
  keepReason: string | undefined,
): void {
  const scanned = scanOptions(tokens, start + 1, false);
  if (!scanned.loaders) {
    // `NODE_OPTIONS='--import tsx' node a.ts` loads tsx without a flag of its own.
    if (keepReason === KEEP_NODE_OPTIONS) {
      rewrite.kept.push(keepReason);
    }
    return;
  }
  const reason = keepReason ?? runReason(rewrite.cwd, tokens, scanned, false);
  if (reason) {
    rewrite.kept.push(reason);
    return;
  }
  rewrite.edits.push(
    { start: tokens[first].start, end: tokens[start].end, text: 'vpx' },
    ...scanned.removals,
  );
}

/** What the options of `tsx` or `node` before the entry say about a run. */
interface ScannedOptions {
  /** Where the entry, or with `--test` the test files, start. */
  entryIndex: number;
  /** Removals of tsx loader flags, and for tsx, of `--no-cache`. */
  removals: Edit[];
  loaders: number;
  /** Local `--require` and `--import` modules, which load before the scripts. */
  preloads: string[];
  keepReason?: string;
  test: boolean;
  watch: boolean;
}

/** Read the options of `tsx` (`tsx` true) or `node` from `tokens[index]` up to the entry. */
function scanOptions(tokens: LiteralCommandToken[], index: number, tsx: boolean): ScannedOptions {
  const scanned: ScannedOptions = {
    entryIndex: tokens.length,
    removals: [],
    loaders: 0,
    preloads: [],
    test: false,
    watch: false,
  };
  for (; index < tokens.length; index++) {
    const { value } = tokens[index];
    if (value === '--' || !value.startsWith('-')) {
      scanned.entryIndex = value === '--' ? index + 1 : index;
      break;
    }
    const name = optionName(value);
    const inline = value.includes('=');
    if (tsx && name === '--no-cache') {
      // `vpx` has no transform cache, and Node.js would reject the flag.
      scanned.removals.push(removal(tokens, index));
      continue;
    }
    if (LOADER_OPTIONS.has(name)) {
      const loader = inline ? value.slice(value.indexOf('=') + 1) : tokens[index + 1]?.value;
      if (loader !== undefined && TSX_LOADER.test(loader)) {
        // A tsx loader would compete with the hooks of `vpx`.
        scanned.removals.push({
          ...removal(tokens, index),
          end: tokens[inline ? index : index + 1].end,
        });
        scanned.loaders++;
      } else if (loader !== undefined && (loader.startsWith('.') || path.isAbsolute(loader))) {
        scanned.preloads.push(loader);
      }
      if (!inline) {
        index++;
      }
      continue;
    }
    scanned.keepReason ??= KEEP_OPTIONS.get(name);
    scanned.test ||= name === '--test';
    scanned.watch ||= name === '--watch';
    if (
      !inline &&
      (NODE_OPTIONS_WITH_VALUE.has(name) ||
        (tsx && (WATCH_OPTIONS_WITH_VALUE.has(name) || name === '--tsconfig')))
    ) {
      index++;
    }
  }
  return scanned;
}

/** Why `vpx` cannot run what the scanned options and the scripts after them run. */
function runReason(
  cwd: string | undefined,
  tokens: LiteralCommandToken[],
  scanned: ScannedOptions,
  watch: boolean,
): string | undefined {
  if (scanned.keepReason) {
    return scanned.keepReason;
  }
  if ((watch || scanned.watch) && scanned.test) {
    return KEEP_WATCH_TEST;
  }
  return runKeepReason(
    cwd,
    scanned.test,
    tokens.slice(scanned.entryIndex).map(({ value }) => value),
    scanned.preloads,
  );
}

/**
 * Rewrite a command that a launcher runs, given as one quoted argument. `inherited` keeps
 * tsx for it, as when the launcher passes on `TSX_TSCONFIG_PATH`.
 */
function rewriteQuotedCommand(
  rewrite: Rewrite,
  token: LiteralCommandToken,
  inherited: string | undefined,
): void {
  const raw = rewrite.command.slice(token.start, token.end);
  const quote = raw[0];
  if ((quote !== '"' && quote !== "'") || raw.length < 2 || !raw.endsWith(quote)) {
    return;
  }
  const inner = rewriteTsxCommand(token.value, rewrite.cwd);
  rewrite.kept.push(...inner.kept);
  if (inner.command === token.value) {
    return;
  }
  if (inherited) {
    rewrite.kept.push(inherited);
  } else if (!inner.command.includes(quote)) {
    rewrite.edits.push({
      start: token.start,
      end: token.end,
      text: `${quote}${inner.command}${quote}`,
    });
  }
}

/** The commands in a scripts or lint-staged record. */
function recordCommands(record: unknown): string[] {
  if (!isPlainRecord(record)) {
    return [];
  }
  return Object.values(record)
    .flatMap((value) => (Array.isArray(value) ? value : [value]))
    .filter((command): command is string => typeof command === 'string');
}

/** The commands a package.json runs: its scripts and lint-staged tasks. */
function packageCommands(pkg: Record<string, unknown>): string[] {
  return [...recordCommands(pkg.scripts), ...recordCommands(pkg['lint-staged'])];
}

function lintStagedJsonFiles(dir: string): string[] {
  return LINT_STAGED_JSON_CONFIG_FILES.map((name) => path.join(dir, name)).filter(
    (file) => fs.existsSync(file) && isJsonFile(file),
  );
}

/**
 * A package that lists tsx outside devDependencies keeps it: its published code or
 * production scripts can need tsx, and `vpx` comes from the vite-plus devDependency.
 */
function ownsTsxAtRuntime(pkg: DependencyBag): boolean {
  return (
    pkg.dependencies?.tsx !== undefined ||
    pkg.optionalDependencies?.tsx !== undefined ||
    pkg.peerDependencies?.tsx !== undefined
  );
}

function packageDirs(rootDir: string, packages?: readonly WorkspacePackage[]): string[] {
  return [rootDir, ...(packages ?? []).map((pkg) => path.join(rootDir, pkg.path))];
}

/**
 * Whether a script, lint-staged task, or `#!/usr/bin/env tsx` shebang runs tsx in a form
 * `vpx` replaces.
 */
export function detectTsxProject(rootDir: string, packages?: readonly WorkspacePackage[]): boolean {
  return packageDirs(rootDir, packages).some((dir) => {
    const pkg = readPackageJsonIfExists(path.join(dir, 'package.json'));
    if (!pkg || ownsTsxAtRuntime(pkg)) {
      return false;
    }
    const commands = [
      ...packageCommands(pkg),
      ...lintStagedJsonFiles(dir).flatMap((file) => recordCommands(readJsonFile(file))),
    ];
    return (
      commands.some((command) => rewriteTsxCommand(command, dir).command !== command) ||
      sourceTreeMatches(dir, (content, file) => {
        const result = rewriteTsxShebang(content, file);
        return result !== undefined && 'content' in result;
      })
    );
  });
}

/** Rewrite each command of a scripts or lint-staged record; returns whether one changed. */
function rewriteCommandRecord(
  record: Record<string, unknown>,
  label: string,
  cwd: string,
  report?: MigrationReport,
): boolean {
  let edited = false;
  for (const [key, value] of Object.entries(record)) {
    const commands: unknown[] = Array.isArray(value) ? value : [value];
    const rewritten = commands.map((command) => {
      if (typeof command !== 'string') {
        return command;
      }
      const result = rewriteTsxCommand(command, cwd);
      for (const reason of result.kept) {
        warnMigration(`Kept tsx in ${label} "${key}": ${reason}.`, report);
      }
      return result.command;
    });
    if (rewritten.some((command, index) => command !== commands[index])) {
      record[key] = Array.isArray(value) ? rewritten : rewritten[0];
      edited = true;
    }
  }
  return edited;
}

/**
 * The `vpx` version of a script file's tsx shebang, or why it keeps tsx; undefined when
 * the file has no `#!/usr/bin/env tsx` shebang.
 */
function rewriteTsxShebang(
  content: string,
  file: string,
): { content: string } | { reason: string } | undefined {
  const match = TSX_ENV_SHEBANG.exec(content);
  if (!match) {
    return undefined;
  }
  const [line, prefix, command] = match;
  // The kernel runs the shebang's command with the file appended.
  const result = rewriteTsxCommand(`${command.trimEnd()} ${SHEBANG_ENTRY}`);
  const reason = result.kept[0] ?? scriptKeepReason(path.dirname(file), `./${path.basename(file)}`);
  if (reason) {
    return { reason };
  }
  const vpxCommand = result.command.slice(0, -` ${SHEBANG_ENTRY}`.length);
  return { content: `${prefix}${vpxCommand}${content.slice(line.length)}` };
}

/** Point `#!/usr/bin/env tsx` shebangs of a package's script files at `vpx`. */
function rewriteTsxShebangs(dir: string, report?: MigrationReport): boolean {
  let changed = false;
  // The callback never reports a match, so the walk visits every file.
  sourceTreeMatches(dir, (content, file) => {
    const result = rewriteTsxShebang(content, file);
    if (result && 'reason' in result) {
      warnMigration(
        `Kept tsx in the shebang of ${displayRelative(file)}: ${result.reason}.`,
        report,
      );
    } else if (result) {
      fs.writeFileSync(file, result.content);
      changed = true;
    }
    return false;
  });
  return changed;
}

/**
 * What still runs or imports tsx under `dir`, or undefined. The scan crosses into nested
 * packages, which can resolve this package's tsx, and reads build output, which can
 * still load it.
 */
function findTsxUse(dir: string): string | undefined {
  let found: string | undefined;
  sourceTreeMatches(
    dir,
    (content, file) => {
      if (LOCKFILES.has(path.basename(file))) {
        return false;
      }
      const uses = fileUsesTsx(content, file);
      if (uses) {
        found = `${displayRelative(file)} still uses tsx`;
      }
      return uses;
    },
    {
      crossPackageBoundaries: true,
      extensions: REFERENCE_EXTENSIONS,
      includeExtensionless: true,
      includePackageReferences: true,
      // Scripts, `imports` aliases, and tool settings such as ava's `nodeArguments`.
      matchesPackage: (packageDir, pkg) => {
        const field = Object.entries(pkg).find(
          ([key, value]) =>
            !IGNORED_FIELDS.has(key) &&
            jsonUsesTsx(value, ['imports', 'lint-staged', 'scripts'].includes(key)),
        )?.[0];
        if (field !== undefined) {
          found = `\`${field}\` in ${displayRelative(path.join(packageDir, 'package.json'))} still uses tsx`;
        }
        return field !== undefined;
      },
      skipDirs: USE_SCAN_SKIP_DIRS,
    },
  );
  return found;
}

/** Whether `file` runs, imports, or loads tsx. */
function fileUsesTsx(content: string, file: string): boolean {
  if (TSX_SHEBANG.test(content)) {
    return true;
  }
  // lint-staged runs a bare `tsx` task too, with the staged files as its arguments.
  const name = path.basename(file);
  if (
    LINT_STAGED_JSON_CONFIG_FILES.some((config) => config === name) &&
    jsonUsesTsx(parseJsonc(content), true)
  ) {
    return true;
  }
  const extension = path.extname(file);
  if (extension === '.json' || extension === '.jsonc') {
    return jsonUsesTsx(parseJsonc(content), false);
  }
  if (CODE_EXTENSIONS.has(extension)) {
    return TSX_CODE_REFERENCE.test(content);
  }
  // An extensionless file can be JavaScript, such as an executable, as well as shell.
  return TSX_TEXT_REFERENCE.test(content) || (extension === '' && TSX_CODE_REFERENCE.test(content));
}

/**
 * Whether a JSON value holds a string that runs or loads tsx: a tsx command with an
 * argument, a loader flag such as `--import=tsx` or `["--import", "tsx"]`, or a subpath
 * such as `tsx/cjs`. A bare `tsx` counts only in `commands` (scripts, lint-staged tasks,
 * and `imports` aliases) and as a loader setting such as Mocha's `"import": "tsx"`;
 * elsewhere it is often a file extension, as in `moduleFileExtensions`.
 */
function jsonUsesTsx(value: unknown, commands: boolean): boolean {
  if (typeof value === 'string') {
    return (commands && TSX_COMMAND.test(value)) || TSX_CONFIG_VALUE.test(value);
  }
  if (Array.isArray(value)) {
    return value.some(
      (item, index) =>
        jsonUsesTsx(item, commands) ||
        (typeof item === 'string' &&
          LOADER_OPTIONS.has(item) &&
          TSX_LOADER.test(String(value[index + 1]))),
    );
  }
  return (
    isPlainRecord(value) &&
    Object.entries(value).some(
      ([key, item]) =>
        jsonUsesTsx(item, commands) ||
        (LOADER_SETTINGS.has(key) &&
          [item].flat().some((loader) => TSX_LOADER.test(String(loader)))),
    )
  );
}

/**
 * Move tsx scripts to `vpx`: rewrite package.json scripts, lint-staged tasks, and
 * `#!/usr/bin/env tsx` shebangs, then remove the tsx devDependency from each package
 * where nothing uses tsx anymore.
 */
export function migrateTsxToVpx(
  rootDir: string,
  packages?: readonly WorkspacePackage[],
  report?: MigrationReport,
): { changed: boolean; dependencyRemoved: boolean } {
  let changed = false;
  const migrated: string[] = [];
  for (const dir of packageDirs(rootDir, packages)) {
    const packageJsonPath = path.join(dir, 'package.json');
    const pkg = readPackageJsonIfExists(packageJsonPath);
    if (!pkg) {
      continue;
    }
    if (ownsTsxAtRuntime(pkg)) {
      if (packageCommands(pkg).some((c) => TSX_COMMAND.test(c))) {
        warnMigration(
          `Kept tsx in ${displayRelative(packageJsonPath)}: it is listed in dependencies, optionalDependencies, or peerDependencies there, and \`vpx\` comes from the vite-plus devDependency.`,
          report,
        );
      }
      continue;
    }
    migrated.push(dir);
    editJsonFile(packageJsonPath, (json) => {
      let edited = false;
      for (const field of ['scripts', 'lint-staged']) {
        const record = json[field];
        if (
          isPlainRecord(record) &&
          rewriteCommandRecord(record, `${displayRelative(packageJsonPath)} ${field}`, dir, report)
        ) {
          edited = true;
        }
      }
      changed ||= edited;
      return edited ? json : undefined;
    });
    for (const file of lintStagedJsonFiles(dir)) {
      editJsonFile(file, (config) => {
        const edited = rewriteCommandRecord(config, displayRelative(file), dir, report);
        changed ||= edited;
        return edited ? config : undefined;
      });
    }
    if (rewriteTsxShebangs(dir, report)) {
      changed = true;
    }
  }

  // Remove the dependency once every package is rewritten: the root's tsx can serve
  // any package, so its scan covers the whole workspace.
  let dependencyRemoved = false;
  for (const dir of migrated) {
    const packageJsonPath = path.join(dir, 'package.json');
    if (readPackageJsonIfExists(packageJsonPath)?.devDependencies?.tsx === undefined) {
      continue;
    }
    const use = findTsxUse(dir);
    if (use) {
      warnMigration(
        `Kept the tsx devDependency in ${displayRelative(packageJsonPath)}: ${use}.`,
        report,
      );
      continue;
    }
    editJsonFile<DependencyBag>(packageJsonPath, (json) => {
      delete json.devDependencies!.tsx;
      return json;
    });
    dependencyRemoved = true;
  }
  if (report && changed) {
    report.tsxMigrated = true;
  }
  return { changed: changed || dependencyRemoved, dependencyRemoved };
}

export async function confirmTsxMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate tsx scripts to vpx?\n  ' +
        styleText(
          'gray',
          "vpx is Vite+'s built-in TypeScript script runner: `tsx <file>` becomes `vpx <file>`, `tsx watch` becomes `vpx --watch`, and the tsx dependency is removed once nothing uses it.",
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('tsx scripts detected. Auto-migrating to vpx...');
  return true;
}
