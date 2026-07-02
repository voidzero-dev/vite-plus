import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { VITE_PLUS_NAME } from './constants.ts';
import { readJsonFile } from './json.ts';
import { fetchNpmResource, getNpmRegistry } from './npm-config.ts';

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

function findOwningPackageJson(resolvedPath: string, packageName: string): string | undefined {
  let currentDir: string;
  try {
    currentDir = fs.statSync(resolvedPath).isDirectory()
      ? resolvedPath
      : path.dirname(resolvedPath);
  } catch {
    return undefined;
  }
  while (currentDir !== path.dirname(currentDir)) {
    const candidate = path.join(currentDir, 'package.json');
    if (fs.existsSync(candidate)) {
      try {
        const candidatePkg = JSON.parse(fs.readFileSync(candidate, 'utf8'));
        if (candidatePkg.name === packageName) {
          return candidate;
        }
      } catch {
        // Keep walking: this may be an unrelated or malformed nested manifest.
      }
    }
    currentDir = path.dirname(currentDir);
  }
  return undefined;
}

function resolvePackageJsonWithNode(
  require: ReturnType<typeof createRequire>,
  packageName: string,
): string | undefined {
  try {
    return require.resolve(`${packageName}/package.json`);
  } catch {
    // Packages with an exports map often do not expose `./package.json`.
  }
  try {
    return findOwningPackageJson(require.resolve(packageName), packageName);
  } catch {
    return undefined;
  }
}

function findPnpApiPath(projectPath: string): string | undefined {
  let currentDir = path.resolve(projectPath);
  while (currentDir !== path.dirname(currentDir)) {
    const candidate = path.join(currentDir, '.pnp.cjs');
    if (fs.existsSync(candidate)) {
      return candidate;
    }
    currentDir = path.dirname(currentDir);
  }
  return undefined;
}

export function detectPackageMetadata(
  projectPath: string,
  packageName: string,
): PackageMetadata | void {
  // Create require from the project path so resolution only searches the
  // project's dependencies, not the global installation's.
  const require = createRequire(path.join(projectPath, 'noop.js'));
  let pkgFilePath = resolvePackageJsonWithNode(require, packageName);
  if (!pkgFilePath) {
    const pnpApiPath = findPnpApiPath(projectPath);
    if (!pnpApiPath) {
      return;
    }
    try {
      const pnpApi = createRequire(pnpApiPath)(pnpApiPath) as {
        resolveToUnqualified: (request: string, issuer: string) => string;
        findPackageLocator?: (location: string) => unknown;
        setup?: () => void;
      };
      const issuer = path.join(projectPath, 'noop.js');
      // The `.pnp.cjs` walk can climb above the project into an unrelated
      // ancestor's stale PnP data. Only trust an API that actually owns the
      // project path; activating a foreign one would install its resolution
      // hooks process-wide and corrupt every later resolution in this run.
      if (pnpApi.findPackageLocator && !pnpApi.findPackageLocator(issuer)) {
        return;
      }
      // Activating the generated API makes archive-backed Yarn cache paths
      // readable through Node's fs implementation as well.
      pnpApi.setup?.();
      const unqualified = pnpApi.resolveToUnqualified(packageName, issuer);
      pkgFilePath = findOwningPackageJson(unqualified, packageName);
      if (!pkgFilePath) {
        pkgFilePath = resolvePackageJsonWithNode(require, packageName);
      }
    } catch {
      return;
    }
  }
  if (!pkgFilePath) {
    return;
  }
  try {
    const pkg = JSON.parse(fs.readFileSync(pkgFilePath, 'utf8'));
    return {
      name: pkg.name,
      version: pkg.version,
      path: path.dirname(pkgFilePath),
    };
  } catch {
    return;
  }
}

/**
 * Read the nearest package.json file from the current directory up to the root directory.
 * @param currentDir - The current directory to start searching from.
 * @returns The package.json content as a JSON object, or null if no package.json is found.
 */
export function readNearestPackageJson(currentDir: string): Record<string, unknown> | null {
  do {
    const packageJsonPath = path.join(currentDir, 'package.json');
    if (fs.existsSync(packageJsonPath)) {
      return readJsonFile(packageJsonPath);
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
 * Check if an npm package exists on its resolved registry.
 * Returns true if the package exists or if the check could not be performed (network error, timeout).
 * Returns false only if the registry definitively responds with 404.
 */
export async function checkNpmPackageExists(packageName: string): Promise<boolean> {
  const atIndex = packageName.indexOf('@', 2);
  const name = atIndex === -1 ? packageName : packageName.slice(0, atIndex);
  const scope = getScopeFromPackageName(name);
  try {
    const response = await fetchNpmResource(`${getNpmRegistry(scope)}/${name}`, {
      method: 'HEAD',
      timeoutMs: 3000,
    });
    return response.status !== 404;
  } catch {
    return true; // Network error or timeout - let the package manager handle it
  }
}
