export const ANSI = {
  green: '\x1b[32m',
  red: '\x1b[31m',
  reset: '\x1b[0m',
  reverse: '\x1b[7m',
  yellow: '\x1b[33m',
};

export function getScreenDimensions() {
  return { top: 0, left: 0, width: process.stdout.columns, height: process.stdout.rows };
}
