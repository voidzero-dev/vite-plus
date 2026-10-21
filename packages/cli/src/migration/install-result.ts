import type { CommandRunSummary } from '../utils/prompts.ts';
import { warnMsg } from '../utils/terminal.ts';
import type { MigrationReport } from './report.ts';

/**
 * Reconcile a CommandRunSummary from `runViteInstall` with the migration's
 * duration counter and exit-code state. `runViteInstall` returns
 * `{ status: 'failed', exitCode }` without throwing; treating that as a success
 * (incrementing duration unconditionally) would let the migration claim
 * "Dependencies installed" while node_modules is desynced from the just-mutated
 * package.json. This helper centralizes the right handling: credit duration on
 * success, warn + flip exitCode on failure, stay silent on skip.
 */
export function handleInstallResult(
  installSummary: CommandRunSummary,
  rootDir: string,
  report: MigrationReport,
  // The pre-migration "initial" install is best-effort: the migration proceeds
  // regardless of its outcome, and a post-migration "final" install runs with
  // `--force` / `--no-frozen-lockfile` as the authoritative recovery. Only that
  // final install's failure should flip `process.exitCode` so a successful
  // recovery yields exit 0; the initial failure is still surfaced via
  // `report.warnings` + the warn message.
  options?: { propagateExitCode?: boolean },
): number {
  if (installSummary.status === 'installed') {
    return installSummary.durationMs;
  }
  if (installSummary.status === 'failed') {
    const exitCode = installSummary.exitCode ?? 1;
    const message = `Dependency installation failed (exit code ${exitCode}). Run \`vp install\` manually in ${rootDir} to resync node_modules.`;
    warnMsg(message);
    report.warnings.push(message);
    if (options?.propagateExitCode !== false) {
      process.exitCode = exitCode;
    }
    return 0;
  }
  return 0;
}
