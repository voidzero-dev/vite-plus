import { spawnSync } from 'node:child_process';
import path from 'node:path';

import { buildCompletionConsumer, resolveCompletionConsumer } from './completion-consumer.ts';

const root = path.resolve(import.meta.dirname, '../../..');
const binary = process.env.VP_COMPLETION_CONSUMER_BIN
  ? resolveCompletionConsumer(root)
  : buildCompletionConsumer(root);
console.error(`Rust completion consumer: ${binary}`);
const result = spawnSync(
  process.execPath,
  [
    'packages/cli/dist/bin.js',
    'test',
    'run',
    '--config',
    'packages/cli/tests/completion/vitest.config.ts',
    ...process.argv.slice(2),
  ],
  {
    cwd: root,
    env: { ...process.env, VP_COMPLETION_CONSUMER_BIN: binary },
    stdio: 'inherit',
  },
);
if (result.error) {
  throw result.error;
}
process.exit(result.status ?? 1);
