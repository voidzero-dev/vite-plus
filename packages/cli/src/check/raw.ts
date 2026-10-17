// Internal runner behind `vp check`: runs the oxfmt/oxlint CLIs in this one
// process, so Vite and the config loader are imported once instead of once per
// spawned tool. `vp check` writes the steps as JSON to stdin and parses the
// step markers this runner appends to stdout and stderr.

import fs from 'node:fs';
import path from 'node:path';
import { text } from 'node:stream/consumers';
import { pathToFileURL } from 'node:url';

import { fmt } from '../resolve-fmt.ts';
import { lint } from '../resolve-lint.ts';

interface RawCheckStep {
  tool: 'fmt' | 'lint';
  args: string[];
  env: Record<string, string>;
}

// Must match `STEP_MARKER` in packages/cli/binding/src/check/raw.rs.
const STEP_MARKER = '\0vp-check-raw:';

const resolvers = { fmt, lint };
// oxlint-disable-next-line typescript/unbound-method -- Restored as-is after each step.
const exit = process.exit;

const steps: RawCheckStep[] = JSON.parse(await text(process.stdin));
let exitCode = 0;
for (const [index, step] of steps.entries()) {
  exitCode = await runStep(step, index);
  // Native tool code writes to fd 1/2 directly, so the markers bypass the
  // Node streams to stay ordered after the tool output.
  const marker = `${STEP_MARKER}${JSON.stringify({ tool: step.tool, exitCode })}\n`;
  fs.writeSync(1, marker);
  fs.writeSync(2, marker);
  // `vp check` stops at the first failing step.
  if (exitCode !== 0) {
    break;
  }
}
// Exit explicitly: a tool may leave handles open (see the oxfmt note below).
exit(exitCode);

async function runStep(step: RawCheckStep, index: number): Promise<number> {
  const { binPath, envs } = await resolvers[step.tool]();
  const restoreEnv = applyEnv({ ...envs, ...step.env });
  // `bin/<tool>` only imports `../dist/cli.js`, which reads `process.argv` and
  // runs on evaluation. The query re-evaluates it when a tool runs twice
  // (fmt after `lint --fix`).
  const cliUrl = pathToFileURL(path.join(path.dirname(path.dirname(binPath)), 'dist', 'cli.js'));
  cliUrl.searchParams.set('vp-check-step', String(index));
  process.argv = [process.argv[0], binPath, ...step.args];
  process.exitCode = undefined;

  const exitCode = await new Promise<NodeJS.Process['exitCode']>((resolve) => {
    const done = () => {
      process.removeListener('beforeExit', done);
      process.exit = exit;
      resolve(process.exitCode);
    };
    // oxfmt does not await its work at module evaluation, so a step ends when
    // the event loop drains.
    process.once('beforeExit', done);
    // On Node versions with a hanging worker bug, oxfmt calls `process.exit()`
    // 50ms after finishing; treat it as the end of the step instead.
    process.exit = ((code) => {
      if (code !== undefined) {
        process.exitCode = code;
      }
      done();
    }) as typeof process.exit;
    import(cliUrl.href).catch((err: unknown) => {
      console.error(err);
      process.exitCode = 1;
    });
  });
  restoreEnv();
  return Number(exitCode ?? 0);
}

// Like the spawned tools' env merging, values already set by the user win.
function applyEnv(envs: Record<string, string>): () => void {
  const added = Object.keys(envs).filter((key) => process.env[key] === undefined);
  for (const key of added) {
    process.env[key] = envs[key];
  }
  return () => {
    for (const key of added) {
      delete process.env[key];
    }
  };
}
