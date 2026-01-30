#!/usr/bin/env node

import { execFileSync } from 'node:child_process';
import { accessSync, chmodSync, constants, existsSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { debuglog } from 'node:util';

const debug = debuglog('vite-plus/global/bin/wrapper');

const __dirname = dirname(fileURLToPath(import.meta.url));
const require = createRequire(import.meta.url);

function getPackageName() {
  const { platform, arch } = process;
  let suffix = '';
  if (platform === 'linux') {
    suffix = '-gnu';
  } else if (platform === 'win32') {
    suffix = '-msvc';
  }
  return `@voidzero-dev/vite-plus-cli-${platform}-${arch}${suffix}`;
}

function getBinaryPath() {
  const binaryName = process.platform === 'win32' ? 'vp.exe' : 'vp';

  // 1. First check for local binary in same directory (local development)
  const localBinaryPath = join(__dirname, binaryName);
  if (existsSync(localBinaryPath)) {
    return localBinaryPath;
  }

  // 2. Find binary from platform-specific optionalDependency
  const packageName = getPackageName();

  // Try to find the binary in node_modules (sibling of this package)
  const nodeModulesPath = join(__dirname, '..', '..', packageName, binaryName);
  if (existsSync(nodeModulesPath)) {
    return nodeModulesPath;
  }

  // Try require.resolve for hoisted dependencies
  try {
    const packagePath = require.resolve(`${packageName}/package.json`);
    const binaryPath = join(dirname(packagePath), binaryName);
    if (existsSync(binaryPath)) {
      return binaryPath;
    }
  } catch {
    // Package not installed, fall back to JS mode
  }

  return null;
}

function ensureExecutable(binaryPath) {
  // Windows doesn't need executable permission check
  if (process.platform === 'win32') {
    return;
  }

  try {
    accessSync(binaryPath, constants.X_OK);
  } catch {
    // Not executable, try to fix permissions
    try {
      chmodSync(binaryPath, 0o755);
    } catch (chmodError) {
      console.error(`Error: Failed to set executable permission on ${binaryPath}`);
      console.error(`  ${chmodError.message}`);
      process.exit(1);
    }
  }
}

const binaryPath = getBinaryPath();

if (binaryPath) {
  // Ensure the binary is executable (auto-fix on non-Windows)
  ensureExecutable(binaryPath);

  // Rust binary mode: execute the native binary
  // Set VITE_GLOBAL_CLI_JS_SCRIPTS_DIR to point to the dist/ directory
  const jsScriptsDir = join(__dirname, '..', 'dist');
  const env = {
    ...process.env,
    VITE_GLOBAL_CLI_JS_SCRIPTS_DIR: jsScriptsDir,
  };
  const args = process.argv.slice(2);
  debug('execFileSync binaryPath: %o', binaryPath);
  debug('execFileSync args: %o', args);
  debug('execFileSync env: %o', env);

  try {
    execFileSync(binaryPath, args, { stdio: 'inherit', env });
  } catch (error) {
    // execFileSync throws on non-zero exit codes, propagate the exit code
    process.exit(error.status ?? 1);
  }
} else {
  // Binary not found - show installation instructions
  const isWindows = process.platform === 'win32';
  const installCommand = isWindows
    ? 'irm https://viteplus.dev/install.ps1 | iex'
    : 'curl -fsSL https://viteplus.dev/install.sh | bash';

  console.error('Error: Vite+ CLI binary not found.');
  console.error('');
  console.error('Please install vite-plus using:');
  console.error(`  ${installCommand}`);
  console.error('');
  console.error('For more information, visit: https://viteplus.dev');
  process.exit(1);
}
