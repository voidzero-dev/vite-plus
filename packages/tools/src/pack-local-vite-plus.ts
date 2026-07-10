// Pack the checkout's publishable Vite+ packages (vite-plus and
// @voidzero-dev/vite-plus-core) so a local npm registry can serve them at the
// checkout version. Used by local-npm-registry.ts for snapshot tests, e2e,
// and standalone development.
//
// Uses node builtins only and erasable TypeScript syntax, so
// local-npm-registry.ts stays runnable with bare `node` from any directory.

import { execFile } from 'node:child_process';
import { existsSync, readdirSync } from 'node:fs';
import path from 'node:path';
import { promisify } from 'node:util';

const execFileAsync = promisify(execFile);

export async function packLocalVitePlusPackages(
  repoRoot: string,
  destination: string,
): Promise<void> {
  for (const sentinel of ['cli/dist/bin.js', 'core/dist']) {
    if (!existsSync(path.join(repoRoot, 'packages', sentinel))) {
      throw new Error(
        `Cannot pack local Vite+ packages: packages/${sentinel} is missing. Run \`pnpm build\` first.`,
      );
    }
  }
  // `vite-plus` packs `binding/*.node` (see its `files` field), and created
  // projects run `vp config` (their `prepare` script) from their OWN
  // installed vite-plus, so the host binding must exist for the installed
  // package to be functional.
  const bindingDir = path.join(repoRoot, 'packages', 'cli', 'binding');
  const hostBindingPrefix = `vite-plus.${process.platform}-${process.arch}`;
  const hasHostBinding = readdirSync(bindingDir).some(
    (entry) => entry.startsWith(hostBindingPrefix) && entry.endsWith('.node'),
  );
  if (!hasHostBinding) {
    throw new Error(
      `Cannot pack local Vite+ packages: no ${hostBindingPrefix}*.node in ${bindingDir}. ` +
        'Run `pnpm bootstrap-cli` (or build the NAPI binding) first.',
    );
  }
  await execFileAsync(
    'pnpm',
    [
      '--filter',
      'vite-plus',
      '--filter',
      '@voidzero-dev/vite-plus-core',
      'pack',
      '--pack-destination',
      destination,
    ],
    { cwd: repoRoot, timeout: 120_000, shell: process.platform === 'win32' },
  );
  const packed = readdirSync(destination).filter((entry) => entry.endsWith('.tgz'));
  if (packed.length !== 2) {
    throw new Error(
      `Expected 2 packed local Vite+ packages in ${destination}, found: ${packed.join(', ') || 'none'}`,
    );
  }
}
