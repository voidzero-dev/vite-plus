import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { parseSync } from '../../../binding/index.js';
import { type WorkspacePackage } from '../../types/index.ts';
import { readNearestPackageJson } from '../../utils/package.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import { SCRIPT_EXTENSIONS, WATCH_OPTIONS_WITH_VALUE } from '../../vpx-script.ts';
import {
  detectScriptRunnerProject,
  migrateScriptRunnerToVpx,
  rewriteScriptRunnerCommand,
  type ScriptRunnerCommandRewrite,
  type ScriptRunnerMigration,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';

// tsx in command position: `tsx a.ts`, `npx tsx`, `--import tsx`, `--import=tsx`,
// `-r tsx/cjs`, `node_modules\.bin\tsx.cmd`, or inside a quoted command, in any case, as
// Windows finds `TSX`. Not `.tsx` files or packages such as `tsx-foo`.
const TSX_COMMAND =
  /(?:^|[\s;&|("'=/\\`])tsx(?:\.(?:cmd|exe|ps1))?(?:\/(?:esm|cjs))?(?=$|[\s;&|)"'`])/i;

const TSX_LOADER = /^tsx(?:\/(?:esm|cjs))?$/;
const KEEP_EVAL = '`vpx` does not run TypeScript passed to --eval or --print';
const KEEP_CHECK = '`vpx` does not transform TypeScript for -c or --check';
const KEEP_REPL = '`vpx` does not start a TypeScript REPL';
const KEEP_TEST_GLOBS =
  '`--test` without files finds `.ts` tests with tsx globs; list the test files for `vpx --test`';
const KEEP_WATCH_PATH = '`--watch-path` has no `vpx` equivalent; use `--include`';
const KEEP_NODE_OPTIONS =
  '`NODE_OPTIONS` loads tsx, whose hooks would compete with those of `vpx`; remove it first';

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
// JSDoc types in JavaScript can depend on tsx's declarations without a runtime import.
const JSDOC_TYPE_TAG =
  /(?:^|\s)@(arg|argument|augments|enum|extends|implements|import|param|prop|property|return|returns|satisfies|template|this|type|typedef)\b\s*/g;
const JSDOC_PARAMETER_NAME = /^[^\s{[=]+(?:\[\])?(?:\.[^\s{[=]+)*/;
const JSDOC_TEMPLATE_PARAMETER_NAME =
  /^(?:(?:in|out|const)\s+)*[$\p{ID_Start}][$\u200C\u200D\p{ID_Continue}]*/u;
const TSX_IMPORT_TYPE = /\bimport\s*\(\s*(['"])tsx(?:\/[^'"]*)?\1\s*(?:,|\))/;
const TSX_JSDOC_IMPORT =
  /^(?:[^\s{},]+\s*,\s*)?(?:\{[^}]*\}|\*\s+as\s+[^\s{},]+|[^\s{},]+)\s+from\s*(['"])tsx(?:\/[^'"]*)?\1/;
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
const TSX_RUNNER: ScriptRunnerMigration = {
  name: 'tsx',
  dependencies: ['tsx'],
  commandPattern: TSX_COMMAND,
  loaderPattern: TSX_LOADER,
  isRegister: (option, value) => option === '--import' && value === 'tsx',
  envShebang: TSX_ENV_SHEBANG,
  shebangPattern: TSX_SHEBANG,
  codeReference: TSX_CODE_REFERENCE,
  codeUsesRunner: codeUsesTsx,
  textReference: TSX_TEXT_REFERENCE,
  configValue: TSX_CONFIG_VALUE,
  keepOptions: KEEP_OPTIONS,
  dropOptions: new Set(['--no-cache']),
  optionsWithValue: new Set([...WATCH_OPTIONS_WITH_VALUE, '--tsconfig']),
  testWithoutFilesReason: KEEP_TEST_GLOBS,
  envKeepReason,
  scriptKeepReason,
};

export type TsxCommandRewrite = ScriptRunnerCommandRewrite;

export function rewriteTsxCommand(command: string, cwd?: string): TsxCommandRewrite {
  return rewriteScriptRunnerCommand(TSX_RUNNER, command, cwd);
}

export function detectTsxProject(rootDir: string, packages?: readonly WorkspacePackage[]): boolean {
  return detectScriptRunnerProject(TSX_RUNNER, rootDir, packages);
}

export function migrateTsxToVpx(
  rootDir: string,
  packages?: readonly WorkspacePackage[],
  report?: MigrationReport,
): { changed: boolean; dependencyRemoved: boolean } {
  const result = migrateScriptRunnerToVpx(TSX_RUNNER, rootDir, packages, report);
  if (report && result.scriptsChanged) {
    report.tsxMigrated = true;
  }
  return { changed: result.changed, dependencyRemoved: result.dependencyRemoved };
}

/**
 * Why environment assignments keep tsx for the command they apply to, or undefined.
 * Names match in any case, as on Windows.
 */
function envKeepReason(assignments: readonly string[]): string | undefined {
  if (
    assignments.some((value) => /^NODE_OPTIONS=/i.test(value) && TSX_COMMAND.test(value.slice(13)))
  ) {
    return KEEP_NODE_OPTIONS;
  }
  return undefined;
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

/** Whether a JSDoc type annotation or import tag needs tsx's declarations. */
function jsdocUsesTsx(value: string): boolean {
  if (!value.startsWith('*')) {
    return false;
  }
  const text = value
    .replaceAll(/^\s*\* ?/gm, '')
    .replaceAll(/```[\s\S]*?(?:```|$)|~~~[\s\S]*?(?:~~~|$)/g, '');
  for (const match of text.matchAll(JSDOC_TYPE_TAG)) {
    const [tag, name] = match;
    if (jsdocTagUsesTsx(name, text.slice(match.index + tag.length))) {
      return true;
    }
  }
  return false;
}

function jsdocTagUsesTsx(name: string, body: string): boolean {
  if (name === 'import') {
    return TSX_JSDOC_IMPORT.test(body);
  }
  // Parameter and property types can follow the name, including an optional
  // name with a default value. Defaults themselves are not type expressions.
  if (/^(?:arg|argument|param|prop|property)$/.test(name) && !body.startsWith('{')) {
    const end = body.startsWith('[')
      ? jsdocDelimitedEnd(body)
      : JSDOC_PARAMETER_NAME.exec(body)?.[0].length;
    if (end === undefined) {
      return false;
    }
    body = body.slice(end).trimStart();
  }
  if (body.startsWith('{')) {
    const end = jsdocDelimitedEnd(body);
    if (end === undefined) {
      return false;
    }
    if (TSX_IMPORT_TYPE.test(body.slice(0, end))) {
      return true;
    }
    body = body.slice(end).trimStart();
  } else if (/^(?:type|this|enum|augments|extends|implements)$/.test(name)) {
    return jsdocUnbracedTypeUsesTsx(body);
  }
  if (name !== 'template') {
    return false;
  }
  // A constraint is followed by a comma-separated list of required names and
  // bracketed defaults. Stop at the first description, not the first name.
  while (body) {
    const hasDefault = body.startsWith('[');
    const end = hasDefault
      ? jsdocDelimitedEnd(body)
      : JSDOC_TEMPLATE_PARAMETER_NAME.exec(body)?.[0].length;
    if (end === undefined) {
      return false;
    }
    if (hasDefault && TSX_IMPORT_TYPE.test(body.slice(0, end))) {
      return true;
    }
    body = body.slice(end).trimStart();
    if (!body.startsWith(',')) {
      return false;
    }
    body = body.slice(1).trimStart();
  }
  return false;
}

/** End of a braced type or bracketed name/default, excluding trailing prose. */
function jsdocDelimitedEnd(body: string): number | undefined {
  const open = body[0];
  const close = open === '{' ? '}' : ']';
  let depth = 0;
  // Quoted types and defaults can contain braces or brackets.
  for (const token of body.matchAll(
    /"(?:\\.|[^"\\])*"|'(?:\\.|[^'\\])*'|`(?:\\.|[^`\\])*`|[{}[\]]/g,
  )) {
    if (token[0] === open) {
      depth++;
    } else if (token[0] === close && --depth === 0) {
      return token.index + 1;
    }
  }
  return undefined;
}

/** Find the complete unbraced type before a tag's description or the next tag. */
function jsdocUnbracedTypeUsesTsx(body: string): boolean {
  if (!TSX_IMPORT_TYPE.test(body)) {
    return false;
  }
  // Wrapping the candidate in parentheses requires exactly one type. Try the
  // longest prefix first so unions and types with spaces are not cut short.
  const boundaries = Array.from(body.matchAll(/\s+|$/g)).toReversed();
  for (const boundary of boundaries) {
    const type = body.slice(0, boundary.index);
    if (parseSync('jsdoc.ts', `type T = (${type});`).errors.length > 0) {
      continue;
    }
    if (TSX_IMPORT_TYPE.test(type)) {
      return true;
    }
    // An unsupported JSDoc type can follow a valid prefix, for example
    // `null | function(import('tsx/esm/api').RegisterOptions): void`.
    const rest = body.slice(boundary.index).trimStart();
    return /^(?:[|&<.([?!:]|extends\b|is\b)/.test(rest) && TSX_IMPORT_TYPE.test(rest);
  }
  // Keep the dependency for JSDoc-only syntax that the TS parser cannot read.
  return true;
}

/** Whether source code or JSDoc types still need tsx. */
function codeUsesTsx(content: string, file: string): boolean {
  // Only parse files with a possible reference. Regex-based comment stripping can
  // mistake a regex literal for a comment and hide a real import of tsx.
  if (!content.includes('tsx')) {
    return false;
  }
  // Framework components need their own parsers. Keep the conservative text scan.
  if (!SCRIPT_EXTENSIONS.includes(path.extname(file))) {
    return TSX_CODE_REFERENCE.test(content);
  }
  const parsed = parseSync(file, content);
  if (parsed.errors.length > 0) {
    return TSX_CODE_REFERENCE.test(content);
  }
  const isJavaScript = /\.(?:[cm]?js|jsx)$/.test(file);
  let codeWithoutComments = '';
  let offset = 0;
  for (const comment of parsed.comments) {
    if (isJavaScript && comment.type === 'Block' && jsdocUsesTsx(comment.value)) {
      return true;
    }
    codeWithoutComments += `${content.slice(offset, comment.start)} `;
    offset = comment.end;
  }
  return TSX_CODE_REFERENCE.test(codeWithoutComments + content.slice(offset));
}

export async function confirmTsxMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate tsx scripts to Vite+?\n  ' +
        styleText(
          'gray',
          '`tsx <file>` becomes `vpx <file>`, `tsx watch` becomes `vpx --watch`, and `node --import tsx` uses `vite-plus/register`. The tsx dependency is removed once nothing uses it.',
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('tsx scripts detected. Auto-migrating to Vite+...');
  return true;
}
