import { type Command, multiplex } from 'multiplexer';
import { join } from 'node:path';

const cwd = process.cwd();
const env = { ...process.env, FORCE_COLOR: 'true' };

const cmds: Command[][] = [];

const getRandom = (max: number) => Math.floor(Math.random() * max) + 1;

for (let i = 0; i < getRandom(3) + 1; i++) {
  cmds.push(
    Array.from({ length: getRandom(4) }, (): Command => {
      const duration = getRandom(4);
      return {
        name: `timer: ${duration}s (${i + 1})`,
        cmd: 'bash',
        args: [join(cwd, 'demo-countdown.sh'), String(duration)],
        cwd,
        env,
      };
    }),
  );
}

await multiplex(cmds);
