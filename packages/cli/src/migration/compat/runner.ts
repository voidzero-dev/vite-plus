import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import type { OxlintConfig } from 'oxlint';

import { runCommandSilently } from '../../utils/command.ts';
import { addMigrationWarning, type MigrationReport } from '../report.ts';
import { LINT_CONFIG_RESULT_PREFIX, ROLLDOWN_COMPAT_RESULT_PREFIX } from './protocol.ts';

export { ROLLDOWN_COMPAT_RESULT_PREFIX };

/**
 * Resolve the isolated compat worker emitted at `migration/compat/worker.js`.
 *
 * In source the worker is a sibling of this module. Bundling can place this
 * runner in `migration/bin.js` or a shared chunk at the dist root, since both
 * migration and creation import config helpers. Check both bundled layouts
 * before falling back to the source layout.
 */
function resolveWorkerPath(): string {
  for (const relativePath of ['./migration/compat/worker.js', './compat/worker.js']) {
    const worker = new URL(relativePath, import.meta.url);
    if (existsSync(worker)) {
      return fileURLToPath(worker);
    }
  }
  return fileURLToPath(new URL('./worker.js', import.meta.url));
}

// Config resolution executes arbitrary project code; on the upgrade path the
// project's own (older) vite-plus copy may not honor the lazyPlugins-skip env
// handshake, so a blocking plugin factory can wedge the worker. Kill it after
// this long; each caller decides how to report the failed config load.
const WORKER_TIMEOUT_MS = 30_000;

interface RolldownCompatibilityResult {
  warnings: string[];
}

function parseWorkerResult(stdout: Buffer, prefix: string): unknown {
  const output = stdout.toString();
  const markerIndex = output.lastIndexOf(prefix);
  if (markerIndex === -1) {
    return undefined;
  }

  const resultStart = markerIndex + prefix.length;
  const resultEnd = output.indexOf('\n', resultStart);
  const serialized = output.slice(resultStart, resultEnd === -1 ? undefined : resultEnd).trim();

  try {
    return JSON.parse(serialized);
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

    const compatibilityResult = parseWorkerResult(result.stdout, ROLLDOWN_COMPAT_RESULT_PREFIX) as
      | Partial<RolldownCompatibilityResult>
      | undefined;
    if (
      !Array.isArray(compatibilityResult?.warnings) ||
      !compatibilityResult.warnings.every((item) => typeof item === 'string')
    ) {
      return;
    }
    for (const warning of compatibilityResult.warnings) {
      addMigrationWarning(report, warning);
    }
  } catch {
    // Config resolution is best-effort. Skip failures silently.
  }
}

/** Read only serializable lint metadata without running project code in migration. */
export async function readLintConfigInWorker(rootDir: string): Promise<OxlintConfig | undefined> {
  const result = await runCommandSilently({
    command: process.execPath,
    args: [resolveWorkerPath(), rootDir, 'lint'],
    cwd: rootDir,
    envs: process.env,
    timeoutMs: WORKER_TIMEOUT_MS,
  });
  if (result.exitCode !== 0) {
    throw new Error(`Config worker exited with code ${result.exitCode}`);
  }
  const metadata = parseWorkerResult(result.stdout, LINT_CONFIG_RESULT_PREFIX) as
    | { lint?: OxlintConfig | null }
    | undefined;
  if (metadata?.lint === null) {
    return undefined;
  }
  if (!metadata?.lint || typeof metadata.lint !== 'object' || Array.isArray(metadata.lint)) {
    throw new Error('Config worker did not return lint metadata');
  }
  return metadata.lint;
}
