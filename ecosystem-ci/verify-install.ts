import { createRequire } from 'node:module';

const require = createRequire(`${process.cwd()}/`);

const expectedVersion = '0.0.0';

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
