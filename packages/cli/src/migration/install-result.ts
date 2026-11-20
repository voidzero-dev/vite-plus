import { downloadPackageManager, getPackageManagerOverride } from '../../binding/index.js';
import { PackageManager } from '../types/index.ts';
import { getVpCommand, runCommandSilently } from '../utils/command.ts';
import type { CommandRunSummary } from '../utils/prompts.ts';
import { warnMsg } from '../utils/terminal.ts';
import type { MigrationReport } from './report.ts';

const PNPM_9_INSTALL_IDLE_TIMEOUT_MS = 5 * 60_000;

/** Bound silent pnpm 9 dependency resolution during non-interactive migration. */
export async function getMigrationInstallIdleTimeoutMs(
  cwd: string,
  interactive: boolean | undefined,
  packageManager: PackageManager | undefined,
  version: string | undefined,
): Promise<number | undefined> {
  if (interactive || process.env.VP_SKIP_INSTALL) {
    return undefined;
  }
  try {
    const override = getPackageManagerOverride();
    if ((override?.name ?? packageManager) !== PackageManager.pnpm) {
      return undefined;
    }
    const probeEnv = { ...process.env, NODE_OPTIONS: undefined };
    let pnpmExecutable: string | undefined;
    if (process.env.VP_CLI_BIN) {
      // The global launcher can select a system binary ahead of the project pin.
      let selectionResult = await runCommandSilently({
        ...getVpCommand(['env', 'current', 'pm', '--json']),
        cwd,
        envs: probeEnv,
      });
      if (selectionResult.exitCode === 2) {
        // Older launchers reject the pm argument but include the selection in the full report.
        selectionResult = await runCommandSilently({
          ...getVpCommand(['env', 'current', '--json']),
          cwd,
          envs: probeEnv,
        });
      }
      const { exitCode, stdout } = selectionResult;
      if (exitCode !== 0) {
        return undefined;
      }
      const { package_manager: selection } = JSON.parse(stdout.toString()) as {
        package_manager?: {
          name: string;
          version: string;
          installed?: boolean;
          bin_paths?: Record<string, string>;
        };
      };
      if (selection?.name !== PackageManager.pnpm) {
        return undefined;
      }
      // env current can see a system binary that failed the version probe.
      // The installer falls back to the package manager resolved for the project.
      const launcherVersion = selection.version === 'unknown' ? undefined : selection.version;
      version = launcherVersion ?? version;
      if (override) {
        // Use the installer provider for a missing or unusable launcher.
        pnpmExecutable =
          launcherVersion && selection.installed ? selection.bin_paths?.pnpm : undefined;
        pnpmExecutable ??= (
          await downloadPackageManager({
            ...override,
            version: launcherVersion ?? override.version,
            cwd,
          })
        ).binPaths.pnpm;
      }
    } else if (override) {
      // The local launcher uses the same provider as this binding.
      pnpmExecutable = (await downloadPackageManager({ ...override, cwd })).binPaths.pnpm;
    }
    if (override) {
      if (!pnpmExecutable) {
        return undefined;
      }
      // The override chooses a launcher. pnpm can then switch to the project pin.
      const { exitCode, stdout } = await runCommandSilently({
        command: pnpmExecutable,
        args: ['--version'],
        cwd,
        envs: probeEnv,
      });
      version = exitCode === 0 ? stdout.toString().trim() : undefined;
    }
    return version?.startsWith('9.') ? PNPM_9_INSTALL_IDLE_TIMEOUT_MS : undefined;
  } catch {
    // Let the install command report any manager selection error.
    return undefined;
  }
}

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
    const message =
      installSummary.failureMessage ??
      `Dependency installation failed (exit code ${exitCode}). Run \`vp install\` manually in ${rootDir} to resync node_modules.`;
    if (!installSummary.failureMessage) {
      warnMsg(message);
    }
    report.warnings.push(message);
    if (options?.propagateExitCode !== false) {
      process.exitCode = exitCode;
    }
    return 0;
  }
  return 0;
}
