import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { readJsonFile } from './json.js';

export function getScopeFromPackageName(packageName: string): string {
  if (packageName.startsWith('@')) {
    return packageName.split('/')[0];
  }
  return '';
}

const require = createRequire(import.meta.url);

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
    const pkgFilePath = require.resolve(`${packageName}/package.json`, { paths: [projectPath] });
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
export function readNearestPackageJson<T = Record<string, any>>(currentDir: string): T | null {
  do {
    const packageJsonPath = path.join(currentDir, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      return readJsonFile<T>(packageJsonPath);
    }
    currentDir = path.dirname(currentDir);
  } while (currentDir !== path.dirname(currentDir));
  return null;
}
