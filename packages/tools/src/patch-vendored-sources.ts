import { execFileSync, spawnSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';

import { readPackageManifest } from './package-manifest.ts';
import { patchOxcNode } from './patch-oxc-node.ts';

export function patchVendoredSources(rootDir: string): void {
  patchOxcNode(rootDir);

  const cwd = join(rootDir, 'rolldown');
  const manifestPath = join(cwd, 'packages/rolldown/package.json');
  const { version } = readPackageManifest(manifestPath);
  if (!version) {
    throw new Error(`Missing version in ${manifestPath}`);
  }
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
