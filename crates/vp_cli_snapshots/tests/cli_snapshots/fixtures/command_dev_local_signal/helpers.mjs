import assert from 'node:assert/strict';
import { on } from 'node:events';
import { existsSync, writeSync } from 'node:fs';
import { delimiter, resolve } from 'node:path';
import { createInterface } from 'node:readline';
import { inspect } from 'node:util';

/** How long a launcher may take to print the line that a probe waits for. */
export const STARTUP_DEADLINE_MS = 15_000;

/**
 * Yield the lines of `stream`, and fail when the deadline passes first.
 * Without a deadline, a launcher that hangs before Vite listens keeps the
 * probe silent until the runner's step timeout stops it.
 */
export async function* linesWithin(stream, ms = STARTUP_DEADLINE_MS) {
  const lines = createInterface({ input: stream });
  const signal = AbortSignal.timeout(ms);
  try {
    for await (const [line] of on(lines, 'line', { signal, close: ['close'] })) {
      yield line;
    }
  } catch (error) {
    if (signal.aborted) {
      throw new Error(`the expected line did not arrive within ${ms} ms`, { cause: error });
    }
    throw error;
  } finally {
    lines.close();
  }
}

/** The project-local `vp` launcher, which the local flavor puts first on PATH. */
export function localLauncher() {
  const launcher = process.env.PATH.split(delimiter)
    .map((directory) => resolve(directory, 'vp'))
    .find(existsSync);
  assert.ok(launcher, 'expected the project-local vp launcher on PATH');
  return launcher;
}

/** Parse the `listening <port> <pid>` line that vite.config.mjs prints. */
export function parseListening(line) {
  const match = /^listening (\d+) (\d+)$/.exec(line);
  return match ? { port: Number(match[1]), pid: Number(match[2]) } : undefined;
}

/**
 * Report a probe failure and stop the probe's process group. The runner starts
 * the probe in its own group, so this also stops a Vite process that outlived
 * its launcher, and the runner does not wait for that process's pipes.
 */
export function failAndStopGroup(error) {
  // Write synchronously, because SIGKILL drops output that is still queued.
  writeSync(2, `${inspect(error)}\n`);
  process.kill(-process.pid, 'SIGKILL');
}
