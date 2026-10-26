import { access, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { setTimeout } from 'node:timers/promises';

const { VP_TEST_BARRIER: barrier, VP_TEST_PARTICIPANT: participant } = process.env;
if (barrier) {
  await writeFile(join(barrier, participant), 'ready');
  const other = participant === 'first' ? 'second' : 'first';
  const deadline = Date.now() + 20_000;
  while (true) {
    try {
      await access(join(barrier, other));
      break;
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      if (Date.now() >= deadline) throw new Error('npm preparation was serialized');
      await setTimeout(10);
    }
  }
}
