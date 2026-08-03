import { constants } from 'node:os';

/** Map a child exit to a shell-compatible code: signal deaths become 128 + signal number. */
export function exitCodeFromClose(code: number | null, signal: NodeJS.Signals | null): number {
  if (code !== null) {
    return code;
  }
  const signalNumber = signal && constants.signals[signal];
  return signalNumber ? 128 + signalNumber : 1;
}
