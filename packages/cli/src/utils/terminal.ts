import { styleText } from 'node:util';

import { vitePlusHeader } from '../../binding/index.js';

export function log(message: string) {
  /* oxlint-disable-next-line no-console */
  console.log(message);
}

/**
 * Emit the Vite+ banner (header line + trailing blank line) to stdout.
 *
 * Silent when:
 * - stdout is piped or redirected (lefthook/husky, `execSync`, CI, pagers) —
 *   the banner's ANSI styling renders inconsistently there.
 * - a git commit-flow hook is running. Direct shell hooks inherit the
 *   terminal for stdout, so the TTY check alone doesn't catch them; git
 *   sets `GIT_INDEX_FILE` for pre-commit / commit-msg / prepare-commit-msg.
 */
export function printHeader() {
  if (!process.stdout.isTTY) {
    return;
  }
  if (process.env.GIT_INDEX_FILE) {
    return;
  }
  log(vitePlusHeader());
  log('');
}

export function accent(text: string) {
  return styleText('blue', text);
}

export function muted(text: string) {
  return styleText('gray', text);
}

export function success(text: string) {
  return styleText('green', text);
}

export function error(text: string) {
  return styleText('red', text);
}

// Standard message prefix functions matching the Rust CLI convention.
// info/note go to stdout (normal output), warn/error go to stderr (diagnostics).

export function infoMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.log(styleText(['blue', 'bold'], 'info:'), msg);
}

export function warnMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.error(styleText(['yellow', 'bold'], 'warn:'), msg);
}

export function errorMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.error(styleText(['red', 'bold'], 'error:'), msg);
}

export function noteMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.log(styleText(['gray', 'bold'], 'note:'), msg);
}
