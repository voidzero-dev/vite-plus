import { styleText } from 'node:util';

import type { CommandRunSummary } from '../utils/prompts.ts';
import { accent, formatDuration, log } from '../utils/terminal.ts';
import { resolveCreateCompletion } from './utils.ts';

export function getNextCommand(projectDir: string, command: string) {
  if (!projectDir || projectDir === '.') {
    return command;
  }
  return `cd ${projectDir} && ${command}`;
}

/**
 * Report how `vp create` finished.
 *
 * Scaffolding, installing and formatting are separate steps, and the last two
 * can fail after the project directory exists. The summary therefore states
 * which steps failed, suggests the command that has to succeed first, and
 * raises the exit code so a script or CI job sees the failure.
 */
export function showCreateSummary(options: {
  description?: string;
  installSummary?: CommandRunSummary;
  fmtSummary?: CommandRunSummary;
  packageManager: string;
  packageManagerVersion: string;
  projectDir: string;
}) {
  const {
    description,
    installSummary,
    fmtSummary,
    packageManager,
    packageManagerVersion,
    projectDir,
  } = options;

  const completion = resolveCreateCompletion({ installSummary, fmtSummary });

  log(
    `${styleText('magenta', '◇')} Scaffolded ${accent(projectDir)}${
      description ? ` with ${description}` : ''
    }`,
  );
  log(
    `${styleText('gray', '•')} Node ${process.versions.node}  ${packageManager} ${packageManagerVersion}`,
  );
  if (installSummary?.status === 'installed') {
    log(
      `${styleText('green', '✓')} Dependencies installed in ${formatDuration(
        installSummary.durationMs,
      )}`,
    );
  }
  // The scaffold itself succeeded, so the directory is still reported as
  // created — but a failed install or format must not be summarized as a
  // project that is ready to run.
  for (const failure of completion.failures) {
    log(`${styleText('red', '✗')} ${failure}`);
  }
  log(
    `${styleText('blue', '→')} Next: ${accent(getNextCommand(projectDir, completion.nextCommand))}`,
  );
  // Only ever raise the exit code: `handleIgnoredBuilds` may already have set
  // it for a gated build that failed to run.
  if (completion.exitCode !== 0) {
    process.exitCode = completion.exitCode;
  }
}
