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
  if (!binName || !['vp', 'vp-dev'].includes(binName)) {
    console.error('Usage: tool install-global-cli <vp|vp-dev>');
    process.exit(1);
  }

  console.log(`Installing global CLI with bin name: ${binName}`);

  if (binName === 'vp') {
    // CI: use original package.json settings
    execSync('npm install -g ./packages/global --force', {
      stdio: 'inherit',
    });
    return;
  }

  // Local development: temporarily modify package.json to avoid conflicts
  const packageJsonPath = path.resolve('packages/global/package.json');
  const originalContent = readFileSync(packageJsonPath, 'utf-8');
  const packageJson = JSON.parse(originalContent);

  packageJson.name = 'vite-plus-cli-dev';
  packageJson.bin = { 'vp-dev': './bin/wrapper.js' };

  try {
    writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
    execSync('npm install -g ./packages/global --force', {
      stdio: 'inherit',
    });
  } finally {
    writeFileSync(packageJsonPath, originalContent);
  }
}
