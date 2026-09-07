import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { dirname, join } from 'node:path';

import { readNearestPackageJson } from './utils/package.ts';

const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';

function checkCoreVersion(packageJsonPath: string, expectedVersion: string): void {
  const pkg = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
    name?: string;
    version?: string;
  };
  if (pkg.name !== CORE_PACKAGE_NAME || pkg.version !== expectedVersion) {
    throw new Error(
      `Expected ${CORE_PACKAGE_NAME}@${expectedVersion}, but found ${pkg.name}@${pkg.version} at ${packageJsonPath}. ` +
        'Run `vp migrate` to align the Vite alias, then run `vp install`.',
    );
  }
}

function checkProjectCoreVersion(
  cwd: string,
  corePackageJsonPath: string,
  expectedVersion: string,
): void {
  // npm can hoist an upstream Vite peer even when the project only declares
  // vite-plus. Validate Vite only when the nearest package declares it.
  const projectPackage = readNearestPackageJson(cwd);
  const declaresVite = [
    'dependencies',
    'devDependencies',
    'optionalDependencies',
    'peerDependencies',
  ].some((field) => Object.hasOwn(projectPackage?.[field] ?? {}, 'vite'));
  if (!declaresVite) {
    return;
  }

  const projectRequire = createRequire(join(cwd, 'package.json'));
  let projectPackageJsonPath: string | undefined;
  try {
    projectPackageJsonPath = projectRequire.resolve('vite/package.json');
  } catch (cause) {
    if ((cause as NodeJS.ErrnoException).code !== 'MODULE_NOT_FOUND') {
      throw cause;
    }
  }
  if (projectPackageJsonPath && projectPackageJsonPath !== corePackageJsonPath) {
    checkCoreVersion(projectPackageJsonPath, expectedVersion);
  }
}

/** Resolve the same core dependency as the CLI's static `vite` re-exports. */
export function resolveCore(
  subpath = '',
  cwd = process.cwd(),
  modulePath = import.meta.url,
): string {
  const cliRequire = createRequire(modulePath);
  const cliPackageJsonPath = cliRequire.resolve('vite-plus/package.json');
  const { version: expectedVersion } = JSON.parse(
    readFileSync(cliPackageJsonPath, 'utf8'),
  ) as { version: string };
  const cliDir = dirname(cliPackageJsonPath);
  let corePackageJsonPath: string;
  try {
    corePackageJsonPath = cliRequire.resolve('vite/package.json');
    let curr = cliDir;
    let isBundled = false;
    while (true) {
      if (corePackageJsonPath.startsWith(join(curr, 'node_modules') + '/')) {
        isBundled = true;
        break;
      }
      const parent = dirname(curr);
      if (parent === curr) break;
      curr = parent;
    }
    if (!isBundled) {
      throw new Error('Resolved outside CLI directory');
    }
  } catch (cause) {
    throw new Error('Could not resolve the bundled Vite dependency. Run `vp install`.', { cause });
  }
  checkCoreVersion(corePackageJsonPath, expectedVersion);
  checkProjectCoreVersion(cwd, corePackageJsonPath, expectedVersion);

  return cliRequire.resolve(`vite${subpath}`);
}
