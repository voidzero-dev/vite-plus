import { existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

import type { OxlintConfig } from 'oxlint';

import { runCommandSilently } from '../../utils/command.ts';
import { parseJsonValue, isObjectValue } from '../../utils/json-value.ts';
import type { JsonObject } from '../../utils/json.ts';
import type { MigrationReport } from '../report.ts';
import { addMigrationWarning } from '../report.ts';
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

// A project's config can crash or block. Both probes use the same child process
// settings and timeout, and their callers decide how to handle failure.
async function runConfigWorker(rootDir: string, ...args: Array<string>): Promise<Buffer> {
  const { exitCode, stdout } = await runCommandSilently({
    command: process.execPath,
    args: [resolveWorkerPath(), rootDir, ...args],
    cwd: rootDir,
    envs: process.env,
    timeoutMs: 30_000,
  });
  if (exitCode !== 0) {
    throw new Error(`Config worker exited with code ${exitCode}`);
  }
  return stdout;
}

function parseWorkerResult(stdout: Buffer, prefix: string): JsonObject | undefined {
  const output = stdout.toString();
  const markerIndex = output.lastIndexOf(prefix);
  if (markerIndex === -1) {
    return undefined;
  }

  const resultStart = markerIndex + prefix.length;
  const resultEnd = output.indexOf('\n', resultStart);
  const serialized = output.slice(resultStart, resultEnd === -1 ? undefined : resultEnd).trim();

  try {
    const result = parseJsonValue(serialized);
    return isObjectValue(result) ? result : undefined;
  } catch {
    return undefined;
  }
}

export async function checkRolldownCompatibility(
  rootDir: string,
  report: MigrationReport,
): Promise<void> {
  try {
    const stdout = await runConfigWorker(rootDir);
    const warnings = parseWorkerResult(stdout, ROLLDOWN_COMPAT_RESULT_PREFIX)?.warnings;
    if (!Array.isArray(warnings) || !warnings.every((item) => typeof item === 'string')) {
      return;
    }
    for (const warning of warnings) {
      addMigrationWarning(report, warning);
    }
  } catch {
    // Config resolution is best-effort. Skip failures silently.
  }
}

/** Read only serializable lint metadata without running project code in migration. */
export async function readLintConfigInWorker(rootDir: string): Promise<OxlintConfig | undefined> {
  const stdout = await runConfigWorker(rootDir, 'lint');
  const lint = parseWorkerResult(stdout, LINT_CONFIG_RESULT_PREFIX)?.lint;
  if (lint === null) {
    return undefined;
  }
  if (typeof lint !== 'object' || Array.isArray(lint)) {
    throw new Error('Config worker did not return lint metadata');
  }
  return lint;
}
