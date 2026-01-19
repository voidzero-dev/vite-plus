import { execSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import path from 'node:path';
import { parseArgs } from 'node:util';

export function installGlobalCli() {
  const { positionals } = parseArgs({
    allowPositionals: true,
    args: process.argv.slice(3),
  });

  const binName = positionals[0];
  if (!binName || !['vp', 'vite'].includes(binName)) {
    console.error('Usage: tool install-global-cli <vp|vite>');
    process.exit(1);
  }

  const packageJsonPath = path.resolve('packages/global/package.json');

  // Read original package.json
  const originalContent = readFileSync(packageJsonPath, 'utf-8');
  const packageJson = JSON.parse(originalContent);

  // Modify based on bin name
  if (binName === 'vp') {
    // Local development: use different package name to avoid conflicts
    packageJson.name = 'vite-plus-cli-dev';
    packageJson.bin = { vp: './bin/vite' };
  } else {
    // CI: keep original settings
    packageJson.name = 'vite-plus-cli';
    packageJson.bin = { vite: './bin/vite' };
  }

  try {
    // Write modified package.json
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');

    // Install globally
    console.log(`Installing global CLI with bin name: ${binName}`);
    execSync('npm install -g ./packages/global --force', {
      stdio: 'inherit',
    });
  } finally {
    // Restore original package.json
    writeFileSync(packageJsonPath, originalContent);
  }
}
