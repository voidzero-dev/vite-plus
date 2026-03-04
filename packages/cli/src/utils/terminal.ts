import { styleText } from 'node:util';

export function log(message: string) {
  /* oxlint-disable-next-line no-console */
  console.log(message);
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
