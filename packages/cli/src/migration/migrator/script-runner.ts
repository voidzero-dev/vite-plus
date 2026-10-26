import fs from 'node:fs';
import path from 'node:path';

import { parse as parseJsonc } from 'jsonc-parser';

import { type LiteralCommandToken, parseLiteralCommands } from '../../../binding/index.js';
import { type WorkspacePackage } from '../../types/index.ts';
import { editJsonFile, isJsonFile, readJsonFile } from '../../utils/json.ts';
import { readNearestPackageJson } from '../../utils/package.ts';
import { displayRelative } from '../../utils/path.ts';
import { NODE_OPTIONS_WITH_VALUE, SCRIPT_EXTENSIONS } from '../../vpx-script.ts';
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

/** Runner-specific behavior; parsing, file edits, and dependency retention are shared. */
export interface ScriptRunnerMigration {
  name: string;
  aliases?: readonly string[];
  watchSubcommand?: boolean;
  dependencies: readonly string[];
  commandPattern: RegExp;
  loaderPattern: RegExp;
  envShebang: RegExp;
  shebangPattern: RegExp;
  includeExtensionlessShebangs?: boolean;
  codeReference: RegExp;
  textReference: RegExp;
  configValue: RegExp;
  referenceFileNamePattern?: RegExp;
  keepOptions: ReadonlyMap<string, string>;
  dropOptions?: ReadonlySet<string>;
  optionsWithValue?: ReadonlySet<string>;
  testWithoutFilesReason: string;
  envKeepReason: (assignments: readonly string[]) => string | undefined;
  scriptKeepReason?: (cwd: string | undefined, script: string) => string | undefined;
  isLoader?: (option: string, value: string) => boolean;
  /** Loader imports that can keep Node.js and use the public registration entry. */
  isRegister?: (option: string, value: string) => boolean;
  registerEnvKeepReason?: (assignments: readonly string[]) => string | undefined;
  registerKeepOptions?: ReadonlyMap<string, string>;
  registerPreloadKeepReason?: (option: string, value: string) => string | undefined;
  registerInlineCodeKeepReason?: (code: string) => string | undefined;
  prepareRun?: (
    args: readonly string[],
    context: {
      cwd: string | undefined;
      environment: readonly string[];
      nodeLoader: boolean;
      executable: string;
    },
  ) => {
    keepReason?: string;
    /** Argument indices after the executable; no value removes that argument. */
    edits?: readonly { index: number; value?: string }[];
  };
  /** Report unsupported uses even when no command can be converted. */
  detectKeptUses?: boolean;
}

const LOADER_OPTIONS = new Set([
  '--import',
  '--loader',
  '--experimental-loader',
  '--require',
  '-r',
]);
const LOADER_SETTINGS = new Set(['import', 'loader', 'require']);
const KEEP_REPL = '`vpx` does not start a TypeScript REPL';
const KEEP_SHELL =
  'the command uses shell syntax that vp migrate does not rewrite, such as pipes, variables, or subshells';
const KEEP_WATCH_TEST = '`vpx --watch` does not run `--test`';
// Stands for the file that the kernel appends to a shebang's command.
const SHEBANG_ENTRY = './shebang.ts';

// package.json fields that name a runner without using it: dependency declarations and prose.
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
// Script files and framework components whose code can import a runner.
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
// Lockfiles name packages without using them.
const LOCKFILES = new Set([
  'bun.lock',
  'npm-shrinkwrap.json',
  'package-lock.json',
  'pnpm-lock.yaml',
  'yarn.lock',
]);
// The scan for remaining runner uses reads build output, which can still load tsx, but not
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

export interface ScriptRunnerCommandRewrite {
  command: string;
  /** Why the runner stays in part of the command. */
  kept: string[];
}

interface Edit {
  start: number;
  end: number;
  text: string;
}

interface Rewrite {
  command: string;
  environment: readonly string[];
  /** The directory the next segment runs in, or undefined when it is not known. */
  cwd?: string;
  edits: Edit[];
  kept: string[];
  /** Package scripts already being checked through an inherited preload. */
  checkingScripts: ReadonlySet<string>;
}

/**
 * Rewrite supported runner and Node.js loader invocations while preserving shell
 * syntax and arguments. Unsupported forms remain, with their reasons in `kept`.
 */
export function rewriteScriptRunnerCommand(
  runner: ScriptRunnerMigration,
  command: string,
  cwd?: string,
  environment: readonly string[] = [],
  checkingScripts: ReadonlySet<string> = new Set(),
): ScriptRunnerCommandRewrite {
  if (!runner.commandPattern.test(command) && !runner.envKeepReason(environment)) {
    return { command, kept: [] };
  }
  const segments = parseLiteralCommands(command, true);
  // The parser rejects `$VAR`; cmd.exe expands `%VAR%` too.
  if (!segments || /%\w+%/.test(command)) {
    return { command, kept: [KEEP_SHELL] };
  }
  const rewrite: Rewrite = { command, cwd, environment, edits: [], kept: [], checkingScripts };
  for (const tokens of segments) {
    const name = tokens[0].value;
    // After `cd` or another shell state change, scripts are no longer checked.
    if (SHELL_STATE_COMMANDS.includes(name) || name === 'chdir') {
      rewrite.cwd = undefined;
      continue;
    }
    rewriteSegment(runner, rewrite, tokens);
  }
  const rewritten = applyEdits(command, rewrite.edits);
  if (!rewrite.kept.length && runner.commandPattern.test(rewritten)) {
    rewrite.kept.push(`vp migrate does not rewrite this use of ${runner.name}`);
  }
  return { command: rewritten, kept: rewrite.kept };
}

function rewriteSegment(
  runner: ScriptRunnerMigration,
  rewrite: Rewrite,
  tokens: LiteralCommandToken[],
): void {
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
  // Inspect simple `env KEY=value command` wrappers. Options can change the
  // environment or working directory, so retain preloads for those forms.
  let unknownEnvironment = false;
  while (values[first] === 'env') {
    first++;
    if (values[first] === '--') {
      first++;
    }
    if (values[first]?.startsWith('-')) {
      unknownEnvironment = true;
      break;
    }
    skipAssignments();
  }
  if (first >= values.length) {
    return;
  }
  // Runner hooks in NODE_OPTIONS must be replaced together with command changes.
  // Environment assignments also reach the commands a launcher runs.
  const assignments = [...rewrite.environment, ...values.slice(0, first)];
  const envReason = runner.envKeepReason(assignments);
  const environment = scanEnvironmentOptions(runner, assignments);
  if (unknownEnvironment) {
    environment.keepReason ??= inheritedOptionsKeepReason(runner);
  }
  environment.keepReason ??= runner.registerEnvKeepReason?.(assignments);
  environment.keepReason ??= environment.preloads
    .map((script) => runner.scriptKeepReason?.(rewrite.cwd, script))
    .find((reason) => reason !== undefined);
  const editStart = rewrite.edits.length;
  const keptStart = rewrite.kept.length;
  const start = first + getCommandStartIndex(values.slice(first));
  // Nested wrappers can reveal another environment command whose assignments
  // and options were not inspected above.
  if (['env', 'cross-env', 'cross-env-shell'].includes(values[start])) {
    environment.keepReason ??= inheritedOptionsKeepReason(runner);
  }
  const launcher = COMMAND_LAUNCHERS.get(values[start]);
  if (values[start] === runner.name || runner.aliases?.includes(values[start])) {
    rewriteRunner(runner, rewrite, tokens, first, start, envReason, assignments);
  } else if (values[start] === 'node') {
    rewriteNodeLoader(runner, rewrite, tokens, first, start, envReason, assignments, environment);
  } else if (shell) {
    for (const token of tokens.slice(start)) {
      rewriteQuotedCommand(runner, rewrite, token, assignments);
    }
  } else if (launcher) {
    for (let index = start + 1; index < tokens.length; index++) {
      if (launcher(values[index - 1])) {
        rewriteQuotedCommand(runner, rewrite, tokens[index], assignments);
      }
    }
  } else if (values.slice(start).some((value) => runner.commandPattern.test(value))) {
    rewrite.kept.push(`vp migrate does not rewrite this use of ${runner.name}`);
  }
  if (!environment.replacements.length) {
    return;
  }
  // All children inherit this assignment. Keep it and their edits together if one
  // still needs the old runner, rather than installing competing loaders.
  const taskStart = ['npm', 'pnpm', 'yarn', 'bun'].includes(values[start]) ? start : first;
  const reason =
    environment.keepReason ??
    packageTaskKeepReason(runner, rewrite, values.slice(taskStart), assignments);
  if (reason && rewrite.kept.length === keptStart) {
    rewrite.kept.push(reason);
  }
  if (rewrite.kept.length !== keptStart) {
    rewrite.edits.length = editStart;
    return;
  }
  for (const replacement of environment.replacements) {
    const index = replacement.index - rewrite.environment.length;
    if (index >= 0) {
      rewrite.edits.push(literalEdit(rewrite.command, tokens[index], replacement.value));
    }
  }
}

function inheritedOptionsKeepReason(runner: ScriptRunnerMigration): string {
  return `\`NODE_OPTIONS\` loads ${runner.name} into a child command that vp migrate cannot check`;
}

/** Check package scripts that inherit a preload without changing their definitions here. */
function taskKeepReason(
  runner: ScriptRunnerMigration,
  rewrite: Rewrite,
  name: string | undefined,
  inherited: readonly string[],
  lifecycle: boolean,
): string | undefined {
  if (!name || !rewrite.cwd) {
    return inheritedOptionsKeepReason(runner);
  }
  const scripts = readNearestPackageJson(rewrite.cwd)?.scripts;
  if (!isPlainRecord(scripts) || typeof scripts[name] !== 'string') {
    return inheritedOptionsKeepReason(runner);
  }
  const names = lifecycle ? [`pre${name}`, name, `post${name}`] : [name];
  for (const task of names) {
    const command = scripts[task];
    if (typeof command !== 'string') {
      continue;
    }
    const key = `${rewrite.cwd}\0${task}`;
    if (rewrite.checkingScripts.has(key)) {
      return inheritedOptionsKeepReason(runner);
    }
    const checking = new Set([...rewrite.checkingScripts, key]);
    // The task has its own migration. Check that result under the inherited
    // preload too: `node file.cts` can need the runner without naming it in the task.
    const migrated = rewriteScriptRunnerCommand(runner, command, rewrite.cwd, [], checking);
    const reason =
      migrated.kept[0] ??
      rewriteScriptRunnerCommand(runner, migrated.command, rewrite.cwd, inherited, checking)
        .kept[0];
    if (reason) {
      return `script \`${task}\` needs ${runner.name}: ${reason}`;
    }
  }
  return undefined;
}

function packageTaskKeepReason(
  runner: ScriptRunnerMigration,
  rewrite: Rewrite,
  values: readonly string[],
  inherited: readonly string[],
): string | undefined {
  const [manager, command] = values;
  if (!['npm', 'pnpm', 'yarn', 'bun'].includes(manager)) {
    return undefined;
  }
  if (['exec', 'x', 'dlx'].includes(command)) {
    return undefined;
  }
  const explicitRun = command === 'run' || command === 'run-script';
  const name = explicitRun ? values[2] : command;
  const expectedLength = explicitRun ? 3 : 2;
  // Includes lifecycle commands and script shorthands. Extra arguments can select
  // another workspace or supply an entry that the script itself does not name.
  if (values.length !== expectedLength || name?.startsWith('-')) {
    return inheritedOptionsKeepReason(runner);
  }
  return taskKeepReason(runner, rewrite, name, inherited, true);
}

function applyEdits(command: string, pending: readonly Edit[]): string {
  let rewritten = command;
  // An option normalization and a loader removal can cover the same whitespace.
  const edits: Edit[] = [];
  for (const edit of pending.toSorted((a, b) => b.start - a.start)) {
    const previous = edits.at(-1);
    if (previous?.text === '' && edit.text === '' && edit.end >= previous.start) {
      previous.start = Math.min(previous.start, edit.start);
      previous.end = Math.max(previous.end, edit.end);
    } else {
      edits.push({ ...edit });
    }
  }
  for (const edit of edits) {
    rewritten = rewritten.slice(0, edit.start) + edit.text + rewritten.slice(edit.end);
  }
  return rewritten;
}

/** Preserve the surrounding quote style when replacing one complete shell token. */
function literalEdit(command: string, token: LiteralCommandToken, value: string): Edit {
  const raw = command.slice(token.start, token.end);
  const assignment = /^([A-Za-z_]\w*=)/.exec(raw);
  const prefix = assignment?.[1] ?? '';
  const quote = raw[prefix.length];
  const preserveQuote = prefix !== '' || raw.endsWith(quote);
  let content = value.slice(prefix.length);
  if (quote === "'" && preserveQuote) {
    content = `'${content.replaceAll("'", "'\\''")}'`;
  } else if ((quote === '"' && preserveQuote) || !/^[\w./=:-]+$/.test(content)) {
    content = `"${content.replaceAll(/[\\"$`]/g, '\\$&')}"`;
  }
  return { start: token.start, end: token.end, text: `${prefix}${content}` };
}

interface EnvironmentOptions {
  replacements: { index: number; value: string }[];
  preloads: string[];
  keepReason?: string;
}

/** NODE_OPTIONS uses double quotes and spaces, not shell single quotes or tab separators. */
function scanEnvironmentOptions(
  runner: ScriptRunnerMigration,
  assignments: readonly string[],
): EnvironmentOptions {
  const result: EnvironmentOptions = { replacements: [], preloads: [] };
  for (const [index, assignment] of assignments.entries()) {
    if (!/^NODE_OPTIONS=/i.test(assignment)) {
      continue;
    }
    const value = assignment.slice(13);
    const shellOnlySyntax = value.includes("'") || value.replaceAll(/"[^"]*"/g, '').includes('\t');
    const segments = shellOnlySyntax ? undefined : parseLiteralCommands(value, false);
    if (!segments || segments.length !== 1) {
      if (runner.commandPattern.test(value)) {
        result.keepReason =
          runner.envKeepReason([assignment]) ??
          `NODE_OPTIONS loads ${runner.name} hooks that vp migrate cannot replace`;
      }
      continue;
    }
    const scanned = scanOptions(runner, segments[0], 0, false);
    result.preloads.push(...scanned.preloads);
    result.keepReason ??= scanned.registerKeepReason;
    if (scanned.loaders !== scanned.imports.length || scanned.entryIndex !== segments[0].length) {
      if (runner.commandPattern.test(value)) {
        result.keepReason =
          runner.envKeepReason([assignment]) ??
          `NODE_OPTIONS loads ${runner.name} hooks that vp migrate cannot replace`;
      }
      continue;
    }
    if (scanned.imports.length) {
      result.replacements.push({
        index,
        value: `${assignment.slice(0, 13)}${applyEdits(value, importEdits(value, scanned))}`,
      });
    }
  }
  return result;
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
  return { start: tokens[index - 1]?.end ?? tokens[index].start, end: tokens[index].end, text: '' };
}

/**
 * Why `vpx` cannot run an invocation: `positionals` are the entry
 * and its arguments, or with `--test`, the test files; `preloads` are local `--require`
 * and `--import` modules, which load first.
 */
function runKeepReason(
  runner: ScriptRunnerMigration,
  cwd: string | undefined,
  test: boolean,
  positionals: readonly string[],
  preloads: readonly string[],
): string | undefined {
  const [entry] = positionals;
  if (entry === undefined) {
    return test ? runner.testWithoutFilesReason : KEEP_REPL;
  }
  // An extensionless entry can be an npm package to `vpx`, rather than a script.
  if (!test && !SCRIPT_EXTENSIONS.some((extension) => entry.endsWith(extension))) {
    return `\`${entry}\` has no file extension, which \`vpx\` needs to run it as a script`;
  }
  const scripts = test ? positionals.filter((value) => !value.startsWith('-')) : [entry];
  for (const script of [...preloads, ...scripts]) {
    const reason = runner.scriptKeepReason?.(cwd, script);
    if (reason) {
      return reason;
    }
  }
  return undefined;
}

function rewriteRunner(
  runner: ScriptRunnerMigration,
  rewrite: Rewrite,
  tokens: LiteralCommandToken[],
  first: number,
  start: number,
  keepReason: string | undefined,
  environment: readonly string[],
): void {
  if (keepReason) {
    rewrite.kept.push(keepReason);
    return;
  }
  const prepared = prepareRun(runner, rewrite, tokens, start, environment, false);
  if (prepared.keepReason) {
    rewrite.kept.push(prepared.keepReason);
    return;
  }
  tokens = prepared.tokens;
  let index = start + 1;
  const watch = runner.watchSubcommand !== false && tokens[index]?.value === 'watch';
  if (watch) {
    index++;
  }
  const scanned = scanOptions(runner, tokens, index, true);
  const reason = runReason(runner, rewrite.cwd, tokens, scanned, watch);
  if (reason) {
    rewrite.kept.push(reason);
    return;
  }
  rewrite.edits.push(
    { start: tokens[first].start, end: tokens[index - 1].end, text: watch ? 'vpx --watch' : 'vpx' },
    ...prepared.edits,
    ...scanned.removals,
  );
}

function rewriteNodeLoader(
  runner: ScriptRunnerMigration,
  rewrite: Rewrite,
  tokens: LiteralCommandToken[],
  first: number,
  start: number,
  keepReason: string | undefined,
  environment: readonly string[],
  environmentOptions: EnvironmentOptions,
): void {
  const original = scanOptions(runner, tokens, start + 1, false);
  if (
    runner.isRegister &&
    original.loaders === original.imports.length &&
    (original.imports.length || environmentOptions.replacements.length)
  ) {
    const prepared = prepareRun(runner, rewrite, tokens, start, environment, true);
    const scanned = scanOptions(runner, prepared.tokens, start + 1, false);
    // Keep Node.js in charge of eval, test discovery, and native watch mode.
    const scripts = scanned.inlineCode
      ? []
      : prepared.tokens.slice(scanned.entryIndex).map(({ value }) => value);
    let reason =
      environmentOptions.keepReason ??
      prepared.keepReason ??
      scanned.registerKeepReason;
    if (
      reason === undefined &&
      environmentOptions.replacements.length &&
      scanned.runScript !== undefined
    ) {
      const name = scanned.entryIndex < prepared.tokens.length ? undefined : scanned.runScript;
      reason = taskKeepReason(runner, rewrite, name, environment, false);
    }
    reason ??= [...scanned.preloads, ...(scanned.test ? scripts : scripts.slice(0, 1))]
      .map((script) => runner.scriptKeepReason?.(rewrite.cwd, script))
      .find((reason) => reason !== undefined);
    if (reason) {
      rewrite.kept.push(reason);
    } else {
      rewrite.edits.push(...prepared.edits, ...importEdits(rewrite.command, scanned));
    }
    return;
  }
  if (!original.loaders) {
    // An unrelated node command in the same script must not receive runner warnings.
    const environmentLoader = environment.some(
      (assignment) =>
        /^NODE_OPTIONS=/i.test(assignment) && runner.commandPattern.test(assignment.slice(13)),
    );
    const reason = original.runnerLoader
      ? original.keepReason
      : environmentLoader
        ? keepReason
        : undefined;
    if (reason) {
      rewrite.kept.push(reason);
    }
    return;
  }
  const prepared = prepareRun(runner, rewrite, tokens, start, environment, true);
  if (prepared.keepReason) {
    rewrite.kept.push(prepared.keepReason);
    return;
  }
  tokens = prepared.tokens;
  const scanned = scanOptions(runner, tokens, start + 1, false);
  if (!scanned.loaders) {
    // A loader can also be supplied through NODE_OPTIONS.
    if (keepReason) {
      rewrite.kept.push(keepReason);
    }
    return;
  }
  const reason = keepReason ?? runReason(runner, rewrite.cwd, tokens, scanned, false);
  if (reason) {
    rewrite.kept.push(reason);
    return;
  }
  rewrite.edits.push(
    { start: tokens[first].start, end: tokens[start].end, text: 'vpx' },
    ...prepared.edits,
    ...scanned.removals,
  );
}

/** Preserve original source ranges while normalizing runner-specific options. */
function prepareRun(
  runner: ScriptRunnerMigration,
  rewrite: Rewrite,
  tokens: LiteralCommandToken[],
  start: number,
  environment: readonly string[],
  nodeLoader: boolean,
): { tokens: LiteralCommandToken[]; edits: Edit[]; keepReason?: string } {
  const prepared = runner.prepareRun?.(
    tokens.slice(start + 1).map(({ value }) => value),
    {
      cwd: rewrite.cwd,
      environment,
      nodeLoader,
      executable: tokens[start].value,
    },
  );
  const edits: Edit[] = [];
  const changes = new Map(prepared?.edits?.map((edit) => [start + 1 + edit.index, edit]));
  const normalized = tokens.flatMap((token, index) => {
    const change = changes.get(index);
    if (!change) {
      return [token];
    }
    if (change.value === undefined) {
      edits.push(removal(tokens, index));
      return [];
    }
    const text = /^[\w./:=@+-]+$/.test(change.value)
      ? change.value
      : `'${change.value.replaceAll("'", "'\\''")}'`;
    edits.push({ start: token.start, end: token.end, text });
    return [{ ...token, value: change.value }];
  });
  return { tokens: normalized, edits, keepReason: prepared?.keepReason };
}

/** The options before the script entry. */
interface ScannedOptions {
  /** Where the entry, or with `--test` the test files, start. */
  entryIndex: number;
  /** Runner-specific options and loader flags to remove. */
  removals: Edit[];
  loaders: number;
  runnerLoader: boolean;
  imports: { token: LiteralCommandToken; value: string }[];
  registerKeepReason?: string;
  inlineCode: boolean;
  runScript?: string;
  /** Local `--require` and `--import` modules, which load before the scripts. */
  preloads: string[];
  keepReason?: string;
  test: boolean;
  watch: boolean;
}

function importEdits(command: string, scanned: ScannedOptions): Edit[] {
  return scanned.imports.map(({ token, value }) => literalEdit(command, token, value));
}

/** Read options from the executable to the entry file. */
function scanOptions(
  runner: ScriptRunnerMigration,
  tokens: LiteralCommandToken[],
  index: number,
  directRunner: boolean,
): ScannedOptions {
  const scanned: ScannedOptions = {
    entryIndex: tokens.length,
    removals: [],
    loaders: 0,
    runnerLoader: false,
    imports: [],
    inlineCode: false,
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
    if (directRunner && runner.dropOptions?.has(name)) {
      // The profile declares this option redundant for vpx.
      scanned.removals.push(removal(tokens, index));
      continue;
    }
    if (LOADER_OPTIONS.has(name)) {
      const loader = inline ? value.slice(value.indexOf('=') + 1) : tokens[index + 1]?.value;
      if (loader !== undefined && runner.loaderPattern.test(loader)) {
        scanned.runnerLoader = true;
        if (runner.isLoader && !runner.isLoader(name, loader)) {
          scanned.keepReason ??= `vp migrate does not rewrite ${name} ${loader}`;
          scanned.registerKeepReason ??= scanned.keepReason;
          if (!inline) {
            index++;
          }
          continue;
        }
        // The old loader would compete with the hooks of `vpx`.
        scanned.removals.push({
          ...removal(tokens, index),
          end: tokens[inline ? index : index + 1].end,
        });
        scanned.loaders++;
        if (runner.isRegister?.(name, loader)) {
          scanned.imports.push({
            token: tokens[inline ? index : index + 1],
            value: inline
              ? `${value.slice(0, value.indexOf('=') + 1)}vite-plus/register`
              : 'vite-plus/register',
          });
        }
      } else if (loader !== undefined) {
        scanned.registerKeepReason ??= runner.registerPreloadKeepReason?.(name, loader);
        if (loader.startsWith('.') || path.isAbsolute(loader)) {
          scanned.preloads.push(loader);
        }
      }
      if (!inline) {
        index++;
      }
      continue;
    }
    scanned.keepReason ??= runner.keepOptions.get(name);
    scanned.registerKeepReason ??= runner.registerKeepOptions?.get(name);
    if (runner.registerInlineCodeKeepReason) {
      const code = ['-e', '-p', '-pe', '--eval', '--print'].includes(name)
        ? inline
          ? value.slice(value.indexOf('=') + 1)
          : tokens[index + 1]?.value
        : /^-(?:pe|ep|e|p)(.+)$/.exec(value)?.[1];
      if (code !== undefined) {
        scanned.registerKeepReason ??= runner.registerInlineCodeKeepReason(code);
      }
    }
    scanned.test ||= name === '--test';
    scanned.watch ||= name === '--watch';
    scanned.inlineCode ||= ['-e', '-p', '-pe', '--eval', '--print', '--run'].includes(name);
    if (name === '--run') {
      scanned.runScript = inline
        ? value.slice(value.indexOf('=') + 1)
        : (tokens[index + 1]?.value ?? '');
    }
    if (
      !inline &&
      (NODE_OPTIONS_WITH_VALUE.has(name) || (directRunner && runner.optionsWithValue?.has(name)))
    ) {
      index++;
    }
  }
  return scanned;
}

/** Why `vpx` cannot run what the scanned options and the scripts after them run. */
function runReason(
  runner: ScriptRunnerMigration,
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
    runner,
    cwd,
    scanned.test,
    tokens.slice(scanned.entryIndex).map(({ value }) => value),
    scanned.preloads,
  );
}

/** Rewrite a quoted launcher command with the environment it inherits. */
function rewriteQuotedCommand(
  runner: ScriptRunnerMigration,
  rewrite: Rewrite,
  token: LiteralCommandToken,
  environment: readonly string[],
): void {
  const raw = rewrite.command.slice(token.start, token.end);
  const quote = raw[0];
  if ((quote !== '"' && quote !== "'") || raw.length < 2 || !raw.endsWith(quote)) {
    return;
  }
  const inner = rewriteScriptRunnerCommand(
    runner,
    token.value,
    rewrite.cwd,
    environment,
    rewrite.checkingScripts,
  );
  rewrite.kept.push(...inner.kept);
  if (inner.command !== token.value) {
    rewrite.edits.push(literalEdit(rewrite.command, token, inner.command));
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
 * A package that lists a runner outside devDependencies keeps it: published code or
 * production scripts can need it, and `vpx` comes from the vite-plus devDependency.
 */
function ownsRunnerAtRuntime(runner: ScriptRunnerMigration, pkg: DependencyBag): boolean {
  return runner.dependencies.some(
    (dependency) =>
      pkg.dependencies?.[dependency] !== undefined ||
      pkg.optionalDependencies?.[dependency] !== undefined ||
      pkg.peerDependencies?.[dependency] !== undefined,
  );
}

function packageDirs(rootDir: string, packages?: readonly WorkspacePackage[]): string[] {
  return [rootDir, ...(packages ?? []).map((pkg) => path.join(rootDir, pkg.path))];
}

/**
 * Detect supported invocations, or retained uses when the profile requests warnings.
 */
export function detectScriptRunnerProject(
  runner: ScriptRunnerMigration,
  rootDir: string,
  packages?: readonly WorkspacePackage[],
): boolean {
  return packageDirs(rootDir, packages).some((dir) => {
    const pkg = readPackageJsonIfExists(path.join(dir, 'package.json'));
    if (!pkg || (!runner.detectKeptUses && ownsRunnerAtRuntime(runner, pkg))) {
      return false;
    }
    const commands = [
      ...packageCommands(pkg),
      ...lintStagedJsonFiles(dir).flatMap((file) => recordCommands(readJsonFile(file))),
    ];
    return (
      commands.some((command) =>
        runner.detectKeptUses
          ? runner.commandPattern.test(command)
          : rewriteScriptRunnerCommand(runner, command, dir).command !== command,
      ) ||
      (runner.detectKeptUses && findRunnerUse(runner, dir) !== undefined) ||
      sourceTreeMatches(
        dir,
        (content, file) => {
          const result = rewriteRunnerShebang(runner, content, file, dir);
          return result !== undefined && 'content' in result;
        },
        { includeExtensionless: runner.includeExtensionlessShebangs },
      )
    );
  });
}

/** Rewrite each command of a scripts or lint-staged record; returns whether one changed. */
function rewriteCommandRecord(
  runner: ScriptRunnerMigration,
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
      const result = rewriteScriptRunnerCommand(runner, command, cwd);
      for (const reason of result.kept) {
        warnMigration(`Kept ${runner.name} in ${label} "${key}": ${reason}.`, report);
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
 * The migrated runner or Node.js preload shebang, or its retention reason.
 */
function rewriteRunnerShebang(
  runner: ScriptRunnerMigration,
  content: string,
  file: string,
  packageDir: string,
): { content: string } | { reason: string } | undefined {
  const match = runner.envShebang.exec(content);
  if (!match) {
    return undefined;
  }
  const [line, prefix, command] = match;
  // The kernel runs the shebang's command with the file appended.
  // Its working directory belongs to the caller, not to the script's directory.
  const original = `${command} ${SHEBANG_ENTRY}`;
  const result = rewriteScriptRunnerCommand(runner, original);
  if (result.command === original && !result.kept.length) {
    return undefined;
  }
  // Inspect the package's explicit local hooks without treating the package
  // directory as the caller's runtime working directory above.
  const tokens = parseLiteralCommands(command, true)?.[0];
  const preloads =
    tokens?.[0]?.value === 'node' ? scanOptions(runner, tokens, 1, false).preloads : [];
  const reason =
    result.kept[0] ??
    preloads
      .map((script) => runner.scriptKeepReason?.(packageDir, script))
      .find((reason) => reason !== undefined) ??
    runner.scriptKeepReason?.(path.dirname(file), `./${path.basename(file)}`);
  if (reason) {
    return { reason };
  }
  const rewritten = result.command.slice(0, -` ${SHEBANG_ENTRY}`.length);
  // Node.js can run an extensionless entry, while vpx can interpret it as a package.
  if (!path.extname(file) && /^vpx(?:\s|$)/.test(rewritten)) {
    return undefined;
  }
  return { content: `${prefix}${rewritten}${content.slice(line.length)}` };
}

/** Rewrite matching runner and Node.js preload shebangs in a package's scripts. */
function rewriteRunnerShebangs(
  runner: ScriptRunnerMigration,
  dir: string,
  report?: MigrationReport,
): boolean {
  let changed = false;
  // The callback never reports a match, so the walk visits every file.
  sourceTreeMatches(
    dir,
    (content, file) => {
      const result = rewriteRunnerShebang(runner, content, file, dir);
      if (result && 'reason' in result) {
        warnMigration(
          `Kept ${runner.name} in the shebang of ${displayRelative(file)}: ${result.reason}.`,
          report,
        );
      } else if (result) {
        fs.writeFileSync(file, result.content);
        changed = true;
      }
      return false;
    },
    { includeExtensionless: runner.includeExtensionlessShebangs },
  );
  return changed;
}

/**
 * Find a remaining use, including nested packages and build output that can resolve
 * the package's runner dependencies.
 */
function findRunnerUse(runner: ScriptRunnerMigration, dir: string): string | undefined {
  let found: string | undefined;
  sourceTreeMatches(
    dir,
    (content, file) => {
      if (LOCKFILES.has(path.basename(file))) {
        return false;
      }
      const uses = fileUsesRunner(runner, content, file);
      if (uses) {
        found = `${displayRelative(file)} still uses ${runner.name}`;
      }
      return uses;
    },
    {
      crossPackageBoundaries: true,
      extensions: REFERENCE_EXTENSIONS,
      fileNamePattern: runner.referenceFileNamePattern,
      includeExtensionless: true,
      includePackageReferences: true,
      // Scripts, `imports` aliases, and tool settings such as ava's `nodeArguments`.
      matchesPackage: (packageDir, pkg) => {
        const field = Object.entries(pkg).find(
          ([key, value]) =>
            !IGNORED_FIELDS.has(key) &&
            jsonUsesRunner(runner, value, ['imports', 'lint-staged', 'scripts'].includes(key)),
        )?.[0];
        if (field !== undefined) {
          found = `\`${field}\` in ${displayRelative(path.join(packageDir, 'package.json'))} still uses ${runner.name}`;
        }
        return field !== undefined;
      },
      skipDirs: USE_SCAN_SKIP_DIRS,
    },
  );
  return found;
}

/** Whether a file still runs, imports, or loads the runner. */
function fileUsesRunner(runner: ScriptRunnerMigration, content: string, file: string): boolean {
  if (runner.shebangPattern.test(content)) {
    return true;
  }
  // lint-staged appends staged files to bare runner commands.
  const name = path.basename(file);
  if (runner.referenceFileNamePattern?.test(name)) {
    return runner.textReference.test(content);
  }
  if (
    LINT_STAGED_JSON_CONFIG_FILES.some((config) => config === name) &&
    jsonUsesRunner(runner, parseJsonc(content), true)
  ) {
    return true;
  }
  const extension = path.extname(file);
  if (extension === '.json' || extension === '.jsonc') {
    return jsonUsesRunner(runner, parseJsonc(content), false);
  }
  if (CODE_EXTENSIONS.has(extension)) {
    return runner.codeReference.test(content);
  }
  // An extensionless file can be JavaScript, such as an executable, as well as shell.
  return (
    runner.textReference.test(content) || (extension === '' && runner.codeReference.test(content))
  );
}

/** Recognize command strings and loader references in structured configuration. */
function jsonUsesRunner(runner: ScriptRunnerMigration, value: unknown, commands: boolean): boolean {
  if (typeof value === 'string') {
    return (commands && runner.commandPattern.test(value)) || runner.configValue.test(value);
  }
  if (Array.isArray(value)) {
    return value.some(
      (item, index) =>
        jsonUsesRunner(runner, item, commands) ||
        (typeof item === 'string' &&
          LOADER_OPTIONS.has(item) &&
          runner.loaderPattern.test(String(value[index + 1]))),
    );
  }
  return (
    isPlainRecord(value) &&
    Object.entries(value).some(
      ([key, item]) =>
        jsonUsesRunner(runner, item, commands) ||
        (LOADER_SETTINGS.has(key) &&
          [item].flat().some((loader) => runner.loaderPattern.test(String(loader)))),
    )
  );
}

/** Rewrite known command surfaces, then remove dependencies with no remaining uses. */
export function migrateScriptRunnerToVpx(
  runner: ScriptRunnerMigration,
  rootDir: string,
  packages?: readonly WorkspacePackage[],
  report?: MigrationReport,
): { changed: boolean; dependencyRemoved: boolean; scriptsChanged: boolean } {
  let changed = false;
  const migrated: string[] = [];
  for (const dir of packageDirs(rootDir, packages)) {
    const packageJsonPath = path.join(dir, 'package.json');
    const pkg = readPackageJsonIfExists(packageJsonPath);
    if (!pkg) {
      continue;
    }
    if (ownsRunnerAtRuntime(runner, pkg)) {
      if (packageCommands(pkg).some((c) => runner.commandPattern.test(c))) {
        warnMigration(
          `Kept ${runner.name} in ${displayRelative(packageJsonPath)}: it is listed in dependencies, optionalDependencies, or peerDependencies there, and \`vpx\` comes from the vite-plus devDependency.`,
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
          rewriteCommandRecord(
            runner,
            record,
            `${displayRelative(packageJsonPath)} ${field}`,
            dir,
            report,
          )
        ) {
          edited = true;
        }
      }
      changed ||= edited;
      return edited ? json : undefined;
    });
    for (const file of lintStagedJsonFiles(dir)) {
      editJsonFile(file, (config) => {
        const edited = rewriteCommandRecord(runner, config, displayRelative(file), dir, report);
        changed ||= edited;
        return edited ? config : undefined;
      });
    }
    if (rewriteRunnerShebangs(runner, dir, report)) {
      changed = true;
    }
  }

  // Remove dependencies after every package is rewritten: the root can serve
  // any package, so its scan covers the whole workspace.
  let dependencyRemoved = false;
  for (const dir of migrated) {
    const packageJsonPath = path.join(dir, 'package.json');
    const devDependencies = readPackageJsonIfExists(packageJsonPath)?.devDependencies;
    if (!runner.dependencies.some((dependency) => devDependencies?.[dependency] !== undefined)) {
      continue;
    }
    const use = findRunnerUse(runner, dir);
    if (use) {
      warnMigration(
        `Kept the ${runner.dependencies.join(' / ')} devDependency in ${displayRelative(packageJsonPath)}: ${use}.`,
        report,
      );
      continue;
    }
    editJsonFile<DependencyBag>(packageJsonPath, (json) => {
      for (const dependency of runner.dependencies) {
        delete json.devDependencies![dependency];
      }
      return json;
    });
    dependencyRemoved = true;
  }
  return { changed: changed || dependencyRemoved, dependencyRemoved, scriptsChanged: changed };
}
