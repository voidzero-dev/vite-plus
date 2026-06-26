import colors from 'picocolors';

import { shouldPrintVitePlusHeader, vitePlusHeader } from '../../binding/index.js';

export function log(message: string) {
  /* oxlint-disable-next-line no-console */
  console.log(message);
}

/**
 * Emit the Vite+ banner (header line + trailing blank line) to stdout.
 * Gating (non-TTY, git hooks) lives in `shouldPrintVitePlusHeader` on the
 * Rust side so both CLIs stay in sync.
 */
export function printHeader() {
  if (!shouldPrintVitePlusHeader()) {
    return;
  }
  log(vitePlusHeader());
  log('');
}

export function accent(text: string) {
  return colors.blue(text);
}

export function muted(text: string) {
  return colors.gray(text);
}

export function success(text: string) {
  return colors.green(text);
}

export function error(text: string) {
  return colors.red(text);
}

// Standard message prefix functions matching the Rust CLI convention.
// info/note go to stdout (normal output), warn/error go to stderr (diagnostics).

export function infoMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.log(colors.bold(colors.blue('info:')), msg);
}

export function warnMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.error(colors.bold(colors.yellow('warn:')), msg);
}

export function errorMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.error(colors.bold(colors.red('error:')), msg);
}

export function noteMsg(msg: string) {
  /* oxlint-disable-next-line no-console */
  console.log(colors.bold(colors.gray('note:')), msg);
}
