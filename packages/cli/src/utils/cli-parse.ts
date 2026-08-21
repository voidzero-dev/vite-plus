import type { CliParseError } from '../../binding/index.js';
import { errorMsg, printHeader } from './terminal.ts';

type CliParseOutcome<T> =
  | { status: 'ok'; value: T }
  | { status: 'exit'; code: number }
  | { status: 'error'; error: CliParseError };

export function unwrapCliParseOutcome<T>(outcome: CliParseOutcome<T>): T {
  if (outcome.status === 'exit') {
    process.exit(outcome.code);
  }
  if (outcome.status === 'error') {
    printHeader();
    errorMsg(outcome.error.message.replace(/^error:\s*/, ''));
    process.exit(1);
  }
  return outcome.value;
}
