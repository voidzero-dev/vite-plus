import { type CommandRunSummary, runViteFmt } from '../utils/prompts.ts';
import { addMigrationWarning, type MigrationReport } from './report.ts';

type FormatRunner = (
  cwd: string,
  interactive?: boolean,
  paths?: string[],
  options?: { silent?: boolean; command?: string; commandArgs?: string[] },
) => Promise<CommandRunSummary>;

const FORMAT_FAILURE_MESSAGE =
  'Automatic formatting failed. Run `vp fmt` manually after migration.';

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
  format: FormatRunner = runViteFmt,
): Promise<boolean> {
  try {
    const cliEntry = process.argv[1];
    const result = await format(projectRoot, interactive, undefined, {
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
