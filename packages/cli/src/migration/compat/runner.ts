import { fileURLToPath } from 'node:url';

import { runCommandSilently } from '../../utils/command.ts';
import { addMigrationWarning, type MigrationReport } from '../report.ts';
import { ROLLDOWN_COMPAT_RESULT_PREFIX } from './protocol.ts';

export { ROLLDOWN_COMPAT_RESULT_PREFIX };

interface RolldownCompatibilityResult {
  warnings: string[];
}

function parseRolldownCompatibilityResult(stdout: Buffer): RolldownCompatibilityResult | undefined {
  const output = stdout.toString();
  const markerIndex = output.lastIndexOf(ROLLDOWN_COMPAT_RESULT_PREFIX);
  if (markerIndex === -1) {
    return undefined;
  }

  const resultStart = markerIndex + ROLLDOWN_COMPAT_RESULT_PREFIX.length;
  const resultEnd = output.indexOf('\n', resultStart);
  const serialized = output.slice(resultStart, resultEnd === -1 ? undefined : resultEnd).trim();

  try {
    const result = JSON.parse(serialized) as Partial<RolldownCompatibilityResult>;
    if (
      !Array.isArray(result.warnings) ||
      !result.warnings.every((item) => typeof item === 'string')
    ) {
      return undefined;
    }
    return { warnings: result.warnings };
  } catch {
    return undefined;
  }
}

/**
 * Resolve a project's Vite config in a child process before checking it for
 * Rolldown-incompatible options. Config files execute arbitrary project code;
 * isolating them prevents process-level handlers, explicit exits, and
 * asynchronous crashes from terminating the migration itself.
 */
export async function checkRolldownCompatibility(
  rootDir: string,
  report: MigrationReport,
): Promise<void> {
  try {
    const workerPath = fileURLToPath(new URL('./compat/worker.js', import.meta.url));
    const result = await runCommandSilently({
      command: process.execPath,
      args: [workerPath, rootDir],
      cwd: rootDir,
      envs: process.env,
    });

    if (result.exitCode !== 0) {
      return;
    }

    const compatibilityResult = parseRolldownCompatibilityResult(result.stdout);
    for (const warning of compatibilityResult?.warnings ?? []) {
      addMigrationWarning(report, warning);
    }
  } catch {
    // Config resolution is best-effort. Skip failures silently.
  }
}
