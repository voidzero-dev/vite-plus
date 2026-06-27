import fs from 'node:fs';
import path from 'node:path';

import { runCommandSilently } from '../utils/command.ts';
import { type CommandRunSummary, runViteFmt } from '../utils/prompts.ts';
import { addMigrationWarning, type MigrationReport } from './report.ts';

type FormatRunner = (
  cwd: string,
  interactive?: boolean,
  paths?: string[],
  options?: { silent?: boolean; command?: string; commandArgs?: string[] },
) => Promise<CommandRunSummary>;

type FormatPathCollector = (
  cwd: string,
  excludedPaths?: ReadonlySet<string>,
) => Promise<string[] | undefined>;

interface FormatMigratedProjectOptions {
  format?: FormatRunner;
  collectPaths?: FormatPathCollector;
  excludedPaths?: ReadonlySet<string>;
}

const FORMAT_FAILURE_MESSAGE =
  'Automatic formatting failed. Run `vp fmt` manually after migration.';

function parseNullDelimitedPaths(output: Buffer): string[] {
  return output.toString().split('\0').filter(Boolean);
}

function isExistingFile(projectRoot: string, relativePath: string): boolean {
  const absolutePath = path.join(projectRoot, relativePath);
  return fs.existsSync(absolutePath) && fs.statSync(absolutePath).isFile();
}

/**
 * Limit automatic formatting to files changed in the current Git worktree.
 * This prevents migration from reformatting unrelated source trees while still
 * covering manifests, generated config, and rewritten imports.
 *
 * Return `undefined` outside a Git worktree so non-Git projects retain the
 * existing full-project formatting behavior.
 */
export async function collectChangedFormatPaths(
  projectRoot: string,
  excludedPaths?: ReadonlySet<string>,
): Promise<string[] | undefined> {
  try {
    const git = (args: string[]) =>
      runCommandSilently({ command: 'git', args, cwd: projectRoot, envs: process.env });
    const [unstaged, staged, untracked] = await Promise.all([
      git(['diff', '--name-only', '--relative', '-z', '--diff-filter=ACMRTUXB', '--', '.']),
      git([
        'diff',
        '--cached',
        '--name-only',
        '--relative',
        '-z',
        '--diff-filter=ACMRTUXB',
        '--',
        '.',
      ]),
      git(['ls-files', '--others', '--exclude-standard', '-z', '--', '.']),
    ]);
    if (unstaged.exitCode !== 0 || staged.exitCode !== 0 || untracked.exitCode !== 0) {
      return undefined;
    }

    const changedPaths = new Set([
      ...parseNullDelimitedPaths(unstaged.stdout),
      ...parseNullDelimitedPaths(staged.stdout),
      ...parseNullDelimitedPaths(untracked.stdout),
    ]);

    // Oxfmt owns the supported-file list and skips unknown formats. Passing
    // every existing changed file keeps migration aligned as Oxfmt evolves.
    return [...changedPaths]
      .filter((file) => !excludedPaths?.has(file) && isExistingFile(projectRoot, file))
      .toSorted();
  } catch {
    return undefined;
  }
}

/**
 * Do not apply Oxfmt to a project that still uses Prettier. Their formatting
 * rules can conflict, especially when Prettier is enforced through ESLint.
 */
export function canFormatWithOxfmt(
  hasPrettierDependency: boolean,
  prettierMigrated: boolean,
): boolean {
  return !hasPrettierDependency || prettierMigrated;
}

/**
 * Format a successfully migrated project without turning a formatter problem
 * into an unhandled migration failure. The formatter already prints its
 * stdout/stderr when it exits nonzero; the report keeps the manual follow-up
 * visible in the final migration summary.
 */
export async function formatMigratedProject(
  projectRoot: string,
  interactive: boolean,
  report: MigrationReport,
  options: FormatMigratedProjectOptions = {},
): Promise<boolean> {
  const { format = runViteFmt, collectPaths = collectChangedFormatPaths, excludedPaths } = options;
  try {
    const paths = await collectPaths(projectRoot, excludedPaths);
    if (paths?.length === 0) {
      return true;
    }
    const cliEntry = process.argv[1] ? path.resolve(process.cwd(), process.argv[1]) : undefined;
    const result = await format(projectRoot, interactive, paths, {
      silent: false,
      ...(cliEntry
        ? { command: process.execPath, commandArgs: [...process.execArgv, cliEntry] }
        : {}),
    });
    if (result.status === 'formatted') {
      return true;
    }
  } catch {
    // Treat spawn/config failures the same as a formatter nonzero exit. The
    // migration changes are still valid and the user can format them manually.
  }

  addMigrationWarning(report, FORMAT_FAILURE_MESSAGE);
  return false;
}
