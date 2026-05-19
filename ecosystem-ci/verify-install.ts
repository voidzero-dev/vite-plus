import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const require = createRequire(`${process.cwd()}/`);

const cliPkg = require(
  join(dirname(fileURLToPath(import.meta.url)), '..', 'packages', 'cli', 'package.json'),
) as { version: string };
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
