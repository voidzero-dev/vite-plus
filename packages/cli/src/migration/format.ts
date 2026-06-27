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

// Keep each `vp fmt <...paths>` argument list well under the OS command-line
// limit (ARG_MAX is ~256KB on macOS). Migrating a large monorepo can rewrite
// thousands of files, so the path list is split into batches to avoid an E2BIG
// spawn failure that would leave the migrated source unformatted.
const MAX_FORMAT_ARG_BYTES = 100_000;

function chunkPathsByArgLength(paths: string[]): string[][] {
  const chunks: string[][] = [];
  let current: string[] = [];
  let currentBytes = 0;
  for (const filePath of paths) {
    const bytes = Buffer.byteLength(filePath) + 1; // +1 for the argument separator
    if (current.length > 0 && currentBytes + bytes > MAX_FORMAT_ARG_BYTES) {
      chunks.push(current);
      current = [];
      currentBytes = 0;
    }
    current.push(filePath);
    currentBytes += bytes;
  }
  if (current.length > 0) {
    chunks.push(current);
  }
  return chunks;
}

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

    // Only fall back to whole-project formatting when the project is genuinely
    // not a Git worktree. A worktree that exists but cannot enumerate changes
    // (locked repo, mid-rebase, unusual config) must NOT trigger a full-tree
    // reformat that would bury the migration diff.
    const worktree = await git(['rev-parse', '--is-inside-work-tree']);
    if (worktree.exitCode !== 0 || worktree.stdout.toString().trim() !== 'true') {
      return undefined;
    }

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
      // Inside a worktree but Git could not list changes; skip targeted
      // formatting rather than reformatting the entire project.
      return [];
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
    const formatOptions = {
      silent: false,
      ...(cliEntry
        ? { command: process.execPath, commandArgs: [...process.execArgv, cliEntry] }
        : {}),
    };
    // `undefined` means "format the whole project" (single invocation); a path
    // list is batched so a huge monorepo cannot overflow the command line.
    const batches = paths === undefined ? [undefined] : chunkPathsByArgLength(paths);
    let allFormatted = true;
    for (const batch of batches) {
      const result = await format(projectRoot, interactive, batch, formatOptions);
      if (result.status !== 'formatted') {
        allFormatted = false;
        break;
      }
    }
    if (allFormatted) {
      return true;
    }
  } catch {
    // Treat spawn/config failures the same as a formatter nonzero exit. The
    // migration changes are still valid and the user can format them manually.
  }

  addMigrationWarning(report, FORMAT_FAILURE_MESSAGE);
  return false;
}
