import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

import { patchOxcNode } from './patch-oxc-node.ts';

export function patchVendoredSources(rootDir: string): void {
  patchOxcNode(rootDir);

  const cwd = join(rootDir, 'rolldown');
  const { version }: { version: string } = JSON.parse(
    readFileSync(join(cwd, 'packages/rolldown/package.json'), 'utf8'),
  );
  const patch = join(rootDir, 'packages/tools/patches', `rolldown-${version}.patch`);
  if (
    existsSync(patch) &&
    spawnSync('git', ['apply', '--reverse', '--check', patch], { cwd }).status !== 0
  ) {
    execFileSync('git', ['apply', patch], { cwd, stdio: 'inherit' });
  }
}

if (import.meta.main) {
  patchVendoredSources(process.cwd());
}
