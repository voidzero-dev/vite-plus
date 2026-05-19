import { createRequire } from 'node:module';

import cliPkg from '../packages/cli/package.json' with { type: 'json' };

const require = createRequire(`${process.cwd()}/`);

const expectedVersion = cliPkg.version;

try {
  const pkg = require('vite-plus/package.json') as { version: string; name: string };
  if (pkg.version !== expectedVersion) {
    console.error(`✗ vite-plus: expected version ${expectedVersion}, got ${pkg.version}`);
    process.exit(1);
  }
  console.log(`✓ vite-plus@${pkg.version}`);
} catch {
  console.error('✗ vite-plus: not installed');
  process.exit(1);
}
