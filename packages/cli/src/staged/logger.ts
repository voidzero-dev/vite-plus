import { stripVTControlCharacters } from 'node:util';

import type { Logger } from 'lint-staged';

import { log } from '../utils/terminal.ts';

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

export function createStagedLogger(logger: Logger = { ...console, log }): Logger {
  return {
    ...logger,
    log(message, ...args) {
      if (typeof message === 'string' && args.length === 0 && isWorkflowProgress(message)) {
        return;
      }
      logger.log(message, ...args);
    },
  };
}
