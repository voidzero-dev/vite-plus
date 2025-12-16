import { createRequire } from 'node:module';

import { detectPackageMetadata, VITE_PLUS_NAME } from './utils/index.js';

const require = createRequire(import.meta.url);

interface GlobalPackageJson {
  version: string;
}

/**
 * Get the global CLI version from package.json
 */
export function getGlobalVersion(): string {
  const pkg: GlobalPackageJson = require('../package.json');
  return pkg.version;
}

/**
 * Get the local CLI version if installed in the given directory
 */
export function getLocalVersion(cwd: string): string | null {
  const metadata = detectPackageMetadata(cwd, VITE_PLUS_NAME);
  return metadata?.version ?? null;
}

/**
 * Print version information for both local and global CLI
 */
export function printVersion(cwd: string): void {
  const globalVersion = getGlobalVersion();
  const localVersion = getLocalVersion(cwd);

  console.log('Vite+ Version:');
  console.log(`- Local: ${localVersion ? `v${localVersion}` : 'Not found'}`);
  console.log(`- Global: v${globalVersion}`);
}

printVersion(process.cwd());
