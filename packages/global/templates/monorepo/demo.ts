import { type Command, multiplex } from 'multiplexer';
import { join } from 'node:path';

const cwd = process.cwd();
const env = { ...process.env, FORCE_COLOR: 'true' };

const commands: Command[] = [
  {
    name: 'vite',
    cmd: join(cwd, 'apps/spa', 'node_modules/.bin', 'vite'),
    args: ['dev'],
    cwd: join(cwd, 'apps/spa'),
    env,
    mode: 'watch',
  },
  {
    name: 'next',
    cmd: 'pnpm',
    args: ['dev'],
    cwd: join(cwd, 'apps/next'),
    env,
  },
  {
    name: 'vitest',
    cmd: join(cwd, 'packages/logger', 'node_modules/.bin', 'vitest'),
    args: ['--watch'],
    cwd: join(cwd, 'packages/logger'),
    env,
  },
  {
    name: 'vitest',
    cmd: join(cwd, 'packages/logger', 'node_modules/.bin', 'vitest'),
    args: ['run'],
    cwd: join(cwd, 'packages/logger'),
    env,
  },
  {
    name: 'oxlint',
    cmd: 'pnpm',
    args: ['run', '-F', '@repo/logger', 'lint'],
    cwd,
    env,
  },
  {
    name: 'oxlint',
    cmd: 'pnpm',
    args: ['run', '-F', '@repo/logger', 'lint:watch'],
    cwd,
    env,
    mode: 'watch',
  },
  ...Array.from(
    { length: 2 },
    (_, i): Command => ({
      name: `stream ${i + 1}`,
      cmd: 'bash',
      args: [join(cwd, 'demo-stream.sh')],
      cwd,
      env,
    }),
  ),
  {
    name: 'timer: 4s',
    cmd: 'bash',
    args: [join(cwd, 'demo-countdown.sh'), '4'],
    cwd,
    env,
  },
];

multiplex([commands.sort(() => Math.random() - 0.5)]);
