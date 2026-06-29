import { execFileSync } from 'node:child_process';
import { chmodSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';

const runtimeBin = dirname(process.execPath);
const leakedBin = join(runtimeBin, 'runtime-leak-check');

try {
  writeFileSync(leakedBin, '#!/bin/sh\necho leaked\n');
  chmodSync(leakedBin, 0o755);

  try {
    const output = execFileSync('runtime-leak-check', { encoding: 'utf8' }).trim();
    console.log(`runtime bin leaked: ${output}`);
  } catch {
    console.log('runtime bin not leaked');
  }

  for (const tool of ['node', 'npm', 'npx']) {
    console.log(execFileSync('which', [tool], { encoding: 'utf8' }).trim());
  }
} finally {
  rmSync(leakedBin, { force: true });
}
