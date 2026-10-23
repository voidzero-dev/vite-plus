import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { type WorkspacePackage } from '../../types/index.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import { WATCH_OPTIONS_WITH_VALUE } from '../../vpx-script.ts';
import {
  detectScriptRunnerProject,
  migrateScriptRunnerToVpx,
  rewriteScriptRunnerCommand,
  type ScriptRunnerCommandRewrite,
  type ScriptRunnerMigration,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';

// Match commands and every core subpath conservatively. Only --import register is
// converted; other loaders and APIs still need the standalone packages.
const OXNODE_COMMAND =
  /(?:^|[\s;&|("'=/\\`])oxnode(?:\.(?:cmd|exe|ps1))?(?=$|[@\s;&|)"'`])|@oxc-node\/core(?:\/[^\s'"`]+)?/i;
const OXNODE_LOADER = /^@oxc-node\/core(?:\/.*)?$/;
const OXNODE_PACKAGE_REFERENCE = /@oxc-node\/(?:cli|core)(?=$|[\s/'"`])/;
const OXNODE_CODE_REFERENCE = new RegExp(
  [OXNODE_COMMAND.source, OXNODE_PACKAGE_REFERENCE.source].join('|'),
  'i',
);
const OXNODE_TEXT_REFERENCE = new RegExp(
  [OXNODE_COMMAND.source, OXNODE_PACKAGE_REFERENCE.source].join('|'),
  'im',
);
const KEEP_NODE_WATCH =
  'oxnode --watch starts the Node.js watcher, while `vpx --watch` uses its own watcher; use oxnode watch to migrate it';
const KEEP_TRANSFORM_DEPS =
  '`OXC_TRANSFORM_ALL` changes dependency transforms, while the Vite+ loader always transforms only TypeScript dependencies';
const REGISTER_KEEP_OPTIONS = new Map([
  [
    '--env-file',
    'an env file can configure oxc-node hooks or dependency transforms; migrate it manually',
  ],
  [
    '--env-file-if-exists',
    'an env file can configure oxc-node hooks or dependency transforms; migrate it manually',
  ],
  ['--config-file', 'a Node.js config file can load additional hooks; migrate it manually'],
  [
    '--experimental-config-file',
    'a Node.js config file can load additional hooks; migrate it manually',
  ],
  [
    '--experimental-default-config-file',
    'a Node.js config file can load additional hooks; migrate it manually',
  ],
]);

function isOxnodeRegister(option: string, value: string): boolean {
  return option === '--import' && /^@oxc-node\/core\/register(?:\.mjs)?$/.test(value);
}

function oxnodeEnvironmentReason(assignments: readonly string[]): string | undefined {
  for (const assignment of assignments) {
    const separator = assignment.indexOf('=');
    const name = assignment.slice(0, separator).toUpperCase();
    const value = assignment.slice(separator + 1);
    if (name === 'OXC_TRANSFORM_ALL' && value !== 'typescript') {
      return KEEP_TRANSFORM_DEPS;
    }
    if ((name === 'VP_SCRIPT_TSCONFIG' || name === 'TSX_TSCONFIG_PATH') && value) {
      return `\`${name}\` would select a different configuration for the Vite+ loader`;
    }
  }
  return undefined;
}

const OXNODE_RUNNER: ScriptRunnerMigration = {
  name: 'oxnode',
  dependencies: ['@oxc-node/cli', '@oxc-node/core'],
  commandPattern: OXNODE_COMMAND,
  loaderPattern: OXNODE_LOADER,
  isLoader: isOxnodeRegister,
  isRegister: isOxnodeRegister,
  registerEnvKeepReason: oxnodeEnvironmentReason,
  registerKeepOptions: REGISTER_KEEP_OPTIONS,
  registerPreloadKeepReason(option, value) {
    if (option === '--import' && value === 'vite-plus/register') {
      return undefined;
    }
    return 'an additional startup preload can configure oxc-node hooks or dependency transforms; migrate it manually';
  },
  registerInlineCodeKeepReason(code) {
    if (OXNODE_PACKAGE_REFERENCE.test(code)) {
      return 'inline code uses a standalone oxc-node API or loader that does not configure the bundled Vite+ loader';
    }
    return undefined;
  },
  envShebang: /^(#!\s*\S*\benv\s+(?:-S\s+)?)(oxnode(?=[ \t\r\n]|$)[^\r\n]*)/,
  shebangPattern: /^#![^\n]*[\s/]oxnode(?=\s|$)/,
  codeReference: OXNODE_CODE_REFERENCE,
  textReference: OXNODE_TEXT_REFERENCE,
  configValue: OXNODE_TEXT_REFERENCE,
  referenceFileNamePattern: /^\.env(?:\.|$)/,
  keepOptions: new Map([
    ...REGISTER_KEEP_OPTIONS,
    ['-e', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['--eval', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['-p', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['-pe', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['--print', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['-c', '`vpx -c` selects package shell mode instead of Node.js syntax checking'],
    ['--check', '`vpx` does not transform TypeScript for --check'],
    ['-i', '`vpx` does not start a TypeScript REPL'],
    ['--interactive', '`vpx` does not start a TypeScript REPL'],
    ['--watch', KEEP_NODE_WATCH],
    ['--watch-path', KEEP_NODE_WATCH],
    ['--node-help', '`vpx` does not support oxnode --node-help'],
    ['-h', 'oxnode and `vpx` have different help output'],
    ['--help', 'oxnode and `vpx` have different help output'],
    ['-v', 'oxnode and `vpx` have different versions'],
    ['--version', 'oxnode and `vpx` have different versions'],
  ]),
  optionsWithValue: new Set([...WATCH_OPTIONS_WITH_VALUE, '--tsconfig']),
  testWithoutFilesReason: 'list the test files explicitly before migrating to `vpx --test`',
  envKeepReason(assignments) {
    const reason = oxnodeEnvironmentReason(assignments);
    if (reason) {
      return reason;
    }
    for (const assignment of assignments) {
      const separator = assignment.indexOf('=');
      const name = assignment.slice(0, separator).toUpperCase();
      const value = assignment.slice(separator + 1);
      if (name === 'NODE_OPTIONS' && /@oxc-node\/core/.test(value)) {
        return '`NODE_OPTIONS` loads oxc-node hooks that would compete with those of `vpx`; remove it first';
      }
    }
    return undefined;
  },
  scriptKeepReason(cwd, script) {
    if (cwd === undefined || /[*?[{]/.test(script)) {
      return undefined;
    }
    try {
      const file = script.startsWith('~/')
        ? path.join(os.homedir(), script.slice(2))
        : path.resolve(cwd, script);
      const source = fs.readFileSync(file, 'utf8');
      if (OXNODE_PACKAGE_REFERENCE.test(source)) {
        return `\`${script}\` uses a standalone oxc-node API or loader that does not configure the bundled \`vpx\` loader`;
      }
    } catch {
      // Both runners use the same transformer; an unavailable file is not evidence
      // of a runner-specific incompatibility.
    }
    return undefined;
  },
  detectKeptUses: true,
};

export function rewriteOxnodeCommand(command: string, cwd?: string): ScriptRunnerCommandRewrite {
  return rewriteScriptRunnerCommand(OXNODE_RUNNER, command, cwd);
}

export function detectOxnodeProject(
  rootDir: string,
  packages?: readonly WorkspacePackage[],
): boolean {
  return detectScriptRunnerProject(OXNODE_RUNNER, rootDir, packages);
}

export function migrateOxnodeToVpx(
  rootDir: string,
  packages?: readonly WorkspacePackage[],
  report?: MigrationReport,
): { changed: boolean; dependencyRemoved: boolean } {
  const result = migrateScriptRunnerToVpx(OXNODE_RUNNER, rootDir, packages, report);
  if (report && result.scriptsChanged) {
    report.oxnodeMigrated = true;
  }
  return { changed: result.changed, dependencyRemoved: result.dependencyRemoved };
}

export async function confirmOxnodeMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate oxnode scripts to Vite+?\n  ' +
        styleText(
          'gray',
          '`oxnode <file>` becomes `vpx <file>`, and `oxnode watch` becomes `vpx --watch`. Node.js imports use `vite-plus/register`. Standalone oxc-node dependencies are removed once nothing uses them.',
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('oxnode scripts detected. Auto-migrating to Vite+...');
  return true;
}
