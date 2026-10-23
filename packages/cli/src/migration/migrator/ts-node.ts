import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { styleText } from 'node:util';

import * as prompts from '@voidzero-dev/vite-plus-prompts';

import { type WorkspacePackage } from '../../types/index.ts';
import { cancelAndExit } from '../../utils/prompts.ts';
import {
  detectScriptRunnerProject,
  migrateScriptRunnerToVpx,
  prepareTsNodeRun,
  rewriteScriptRunnerCommand,
  type ScriptRunnerCommandRewrite,
  type ScriptRunnerMigration,
  typescriptScriptKeepReason,
} from '../migrator.ts';
import { type MigrationReport } from '../report.ts';

// Include unsupported entrypoints and package subpaths in retention scans.
const TS_NODE_REFERENCE =
  /(?:^|[\s;&|("'=/\\`])ts-node(?:-(?:cwd|esm|script|transpile-only))?(?:\.(?:cmd|exe|ps1))?(?=$|[@/\s;&|)"'`])/i;
const TS_NODE_LOADERS = /^ts-node(?:\/.*)?$/;
const TS_NODE_RUNNER: ScriptRunnerMigration = {
  name: 'ts-node',
  aliases: ['ts-node-esm', 'ts-node-transpile-only'],
  watchSubcommand: false,
  dependencies: ['ts-node'],
  commandPattern: TS_NODE_REFERENCE,
  loaderPattern: TS_NODE_LOADERS,
  isLoader(option, value) {
    return (
      ((option === '-r' || option === '--require' || option === '--import') &&
        /^ts-node\/register(?:\/(?:transpile-only|type-check|files))?$/.test(value)) ||
      ((option === '--loader' || option === '--experimental-loader') &&
        /^ts-node\/esm(?:\/transpile-only)?(?:\.mjs)?$/.test(value))
    );
  },
  envShebang:
    /^(#!\s*\S*\benv\s+(?:-S\s+)?)(ts-node(?:-(?:esm|transpile-only))?(?=[ \t\r\n]|$)[^\r\n]*)/,
  shebangPattern: /^#![^\n]*[\s/]ts-node(?:-(?:cwd|esm|script|transpile-only))?(?=\s|$)/,
  codeReference: TS_NODE_REFERENCE,
  textReference: TS_NODE_REFERENCE,
  configValue: TS_NODE_REFERENCE,
  referenceFileNamePattern: /^\.env(?:\.|$)/,
  keepOptions: new Map([
    ['-e', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['--eval', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['-p', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['--print', '`vpx` does not run TypeScript passed to --eval or --print'],
    ['-c', '`vpx -c` selects package shell mode instead of Node.js syntax checking'],
    ['--check', '`vpx` does not transform TypeScript for --check'],
    ['-i', '`vpx` does not start a TypeScript REPL'],
    ['--interactive', '`vpx` does not start a TypeScript REPL'],
    ['--watch', 'Node.js watch mode and `vpx --watch` use different watchers'],
    ['--watch-path', 'Node.js watch mode and `vpx --watch` use different watchers'],
    ['-h', 'ts-node and `vpx` have different help output'],
    ['--help', 'ts-node and `vpx` have different help output'],
    ['-v', 'ts-node and `vpx` have different versions'],
    ['--version', 'ts-node and `vpx` have different versions'],
  ]),
  optionsWithValue: new Set(['--tsconfig']),
  testWithoutFilesReason: 'list the test files explicitly before migrating to `vpx --test`',
  envKeepReason(assignments) {
    if (
      assignments.some(
        (assignment) =>
          /^NODE_OPTIONS=/i.test(assignment) && TS_NODE_REFERENCE.test(assignment.slice(13)),
      )
    ) {
      return '`NODE_OPTIONS` loads ts-node hooks that would compete with those of `vpx`; remove it first';
    }
    if (assignments.some((assignment) => /^NODE_OPTIONS=.+/i.test(assignment))) {
      return '`NODE_OPTIONS` can change ts-node startup or load configuration preloads; review it before migrating';
    }
    return undefined;
  },
  prepareRun: (args, context) => prepareTsNodeRun(args, context),
  scriptKeepReason(cwd, script) {
    if (cwd !== undefined && !/[*?[{]/.test(script)) {
      try {
        const file = script.startsWith('~/')
          ? path.join(os.homedir(), script.slice(2))
          : path.resolve(cwd, script);
        const source = fs.readFileSync(file, 'utf8').replace(/^#![^\n]*\n?/, '');
        if (TS_NODE_REFERENCE.test(source)) {
          return `\`${script}\` uses a standalone ts-node API or loader that does not configure the bundled \`vpx\` loader`;
        }
      } catch {
        // The shared source check also reports generated entries that do not exist yet.
      }
    }
    return typescriptScriptKeepReason(cwd, script, 'ts-node');
  },
  detectKeptUses: true,
};

export function rewriteTsNodeCommand(command: string, cwd?: string): ScriptRunnerCommandRewrite {
  return rewriteScriptRunnerCommand(TS_NODE_RUNNER, command, cwd);
}

export function detectTsNodeProject(
  rootDir: string,
  packages?: readonly WorkspacePackage[],
): boolean {
  return detectScriptRunnerProject(TS_NODE_RUNNER, rootDir, packages);
}

export function migrateTsNodeToVpx(
  rootDir: string,
  packages?: readonly WorkspacePackage[],
  report?: MigrationReport,
): { changed: boolean; dependencyRemoved: boolean } {
  const result = migrateScriptRunnerToVpx(TS_NODE_RUNNER, rootDir, packages, report);
  if (report && result.scriptsChanged) {
    report.tsNodeMigrated = true;
  }
  return { changed: result.changed, dependencyRemoved: result.dependencyRemoved };
}

export async function confirmTsNodeMigration(interactive: boolean): Promise<boolean> {
  if (interactive) {
    const confirmed = await prompts.confirm({
      message:
        'Migrate ts-node scripts to vpx?\n  ' +
        styleText(
          'gray',
          'Commands with type-checking disabled become `vpx`. Commands that type-check or use custom ts-node settings stay unchanged. The ts-node devDependency is removed once nothing uses it.',
        ),
      initialValue: true,
    });
    if (prompts.isCancel(confirmed)) {
      cancelAndExit();
    }
    return confirmed;
  }
  prompts.log.info('ts-node scripts detected. Auto-migrating supported commands to vpx...');
  return true;
}
