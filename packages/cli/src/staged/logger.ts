import fs from 'node:fs';
import { stripVTControlCharacters } from 'node:util';

import type { Logger } from 'lint-staged';

import { log, warnMsg } from '../utils/terminal.ts';

// lint-staged exposes a text logger, but no task events or custom renderer.
// Hide its task plan and routine Git progress, keeping command results and
// diagnostics. Match whole messages so verbose task output is never filtered.
const workflowSuccessMessages = new Set([
  '✔ Done preparing lint-staged!',
  '✔ Done running tasks for staged files!',
  '✔ Done running tasks for changed files!',
  '✔ Done staging changes from tasks!',
  '✔ Done hiding unstaged changes to partially staged files!',
  '✔ Done restoring untracked files!',
  '✔ Done cleaning up temporary files!',
]);

function isWorkflowProgress(message: string): boolean {
  const text = stripVTControlCharacters(message);
  if (text.includes('\n')) {
    return false;
  }

  return (
    text === '' ||
    text.startsWith('⋯ ') ||
    text.startsWith('    ') ||
    workflowSuccessMessages.has(text) ||
    /^✔ Done backing up original state \([\da-f]+\)!$/.test(text)
  );
}

const UNSTAGED_PATCH_LABEL = 'Unstaged changes have been kept back in a patch file:';

function isUnstagedPatchPath(message: string): boolean {
  const text = stripVTControlCharacters(message).trim();
  return text.length > 0 && !text.includes('\n') && text.endsWith('/unstaged.patch');
}

export function createStagedLogger(logger: Logger = { ...console, log }): Logger {
  let pendingPatchLabel = false;

  return {
    ...logger,
    log(message, ...args) {
      if (typeof message === 'string' && args.length === 0 && isWorkflowProgress(message)) {
        return;
      }
      logger.log(message, ...args);
    },
    warn(message, ...args) {
      if (typeof message !== 'string' || args.length !== 0) {
        logger.warn(message, ...args);
        return;
      }

      const text = stripVTControlCharacters(message);

      if (pendingPatchLabel) {
        pendingPatchLabel = false;
        if (isUnstagedPatchPath(text)) {
          if (fs.existsSync(text.trim())) {
            logger.warn(UNSTAGED_PATCH_LABEL);
            logger.warn(text);
          } else {
            warnMsg(
              'Unstaged changes were restored from the backup stash. No recovery patch remains.',
            );
          }
          return;
        }
        // Not the expected path: flush the buffered label first.
        logger.warn(UNSTAGED_PATCH_LABEL);
        logger.warn(message);
        return;
      }

      if (text.trim() === UNSTAGED_PATCH_LABEL) {
        pendingPatchLabel = true;
        return;
      }

      logger.warn(message, ...args);
    },
  };
}
