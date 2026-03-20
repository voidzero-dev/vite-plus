import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { VITE_PLUS_NAME } from './constants.js';
import { readJsonFile } from './json.js';

export function getScopeFromPackageName(packageName: string): string {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[0];
  }
  return '';
}

interface PackageMetadata {
  name: string;
  version: string;
  path: string;
}

export function detectPackageMetadata(
  projectPath: string,
  packageName: string,
): PackageMetadata | void {
  try {
    // Create require from the project path so resolution only searches
    // the project's node_modules, not the global installation's
    const require = createRequire(path.join(projectPath, 'noop.js'));
    const pkgFilePath = require.resolve(`${packageName}/package.json`);
    const pkg = JSON.parse(fs.readFileSync(pkgFilePath, 'utf8'));
    return {
      name: pkg.name,
      version: pkg.version,
      path: path.dirname(pkgFilePath),
    };
  } catch {
    // ignore MODULE_NOT_FOUND error
    return;
  }
}

/**
 * Read the nearest package.json file from the current directory up to the root directory.
 * @param currentDir - The current directory to start searching from.
 * @returns The package.json content as a JSON object, or null if no package.json is found.
 */
export function readNearestPackageJson<T = Record<string, unknown>>(currentDir: string): T | null {
  do {
    const packageJsonPath = path.join(currentDir, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      return readJsonFile<T>(packageJsonPath);
    }
    currentDir = path.dirname(currentDir);
  } while (currentDir !== path.dirname(currentDir));
  return null;
}

export function hasVitePlusDependency(
  pkg?: {
    dependencies?: Record<string, string>;
    devDependencies?: Record<string, string>;
  } | null,
) {
  return Boolean(pkg?.dependencies?.[VITE_PLUS_NAME] || pkg?.devDependencies?.[VITE_PLUS_NAME]);
}

/**
 * Check if an npm package exists in the public registry.
 * Returns true if the package exists or if the check could not be performed (network error, timeout).
 * Returns false only if the registry definitively responds with 404.
 */
export async function checkNpmPackageExists(packageName: string): Promise<boolean> {
  const atIndex = packageName.indexOf('@', 2);
  const name = atIndex === -1 ? packageName : packageName.slice(0, atIndex);
  try {
    const response = await fetch(`https://registry.npmjs.org/${name}`, {
      method: 'HEAD',
      signal: AbortSignal.timeout(3000),
    });
    return response.status !== 404;
  } catch {
    return true; // Network error or timeout - let the package manager handle it
  }
}
