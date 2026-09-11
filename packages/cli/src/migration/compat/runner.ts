import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import { runCommandSilently } from '../../utils/command.ts';
import { addMigrationWarning, type MigrationReport } from '../report.ts';
import { ROLLDOWN_COMPAT_RESULT_PREFIX } from './protocol.ts';

export { ROLLDOWN_COMPAT_RESULT_PREFIX };

/**
 * Resolve the isolated compat worker emitted at `migration/compat/worker.js`.
 *
 * The worker is a sibling of this module in source (`src/migration/compat/`),
 * so `./worker.js` is the correct relative path. The bundler, however, inlines
 * this runner into the parent `migration/bin.js` entry, which sits one level up
 * from the emitted `migration/compat/worker.js`; there the worker lives in the
 * nested `./compat/` directory. A fixed literal can only satisfy one layout:
 * `./compat/worker.js` doubles to `compat/compat/worker.js` in source, while
 * `./worker.js` misses the bundled worker. Probe for the nested
 * `./compat/worker.js` next to this module to pick the right prefix for
 * whichever layout this code is running in.
 */
function resolveWorkerPath(): string {
  const nestedWorker = new URL('./compat/worker.js', import.meta.url);
  if (existsSync(nestedWorker)) {
    return fileURLToPath(nestedWorker);
  }
  return fileURLToPath(new URL('./worker.js', import.meta.url));
}

// Config resolution executes arbitrary project code; on the upgrade path the
// project's own (older) vite-plus copy may not honor the lazyPlugins-skip env
// handshake, so a blocking plugin factory can wedge the worker. The check is
// best-effort: kill the worker after this long and continue without warnings
// rather than hanging the migration.
const WORKER_TIMEOUT_MS = 30_000;

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
    const workerPath = resolveWorkerPath();
    const result = await runCommandSilently({
      command: process.execPath,
      args: [workerPath, rootDir],
      cwd: rootDir,
      envs: process.env,
      timeoutMs: WORKER_TIMEOUT_MS,
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
