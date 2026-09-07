import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';

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

/** Resolve the same core dependency as the CLI's static `vite` re-exports. */
export function resolveCore(
  subpath = '',
  cwd = process.cwd(),
  modulePath = import.meta.url,
): string {
  const require = createRequire(modulePath);
  // Read the selected CLI's installed manifest. A static JSON import is inlined
  // during the build, before CI and preview packers can stamp a new version.
  const { version: expectedVersion } = JSON.parse(
    readFileSync(require.resolve('vite-plus/package.json'), 'utf8'),
  ) as { version: string };
  let packageJsonPath: string;
  try {
    packageJsonPath = require.resolve('vite/package.json');
  } catch (cause) {
    throw new Error('Could not resolve the bundled Vite dependency. Run `vp install`.', { cause });
  }
  checkCoreVersion(packageJsonPath, expectedVersion);

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
    return require.resolve(`vite${subpath}`);
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
  if (projectPackageJsonPath && projectPackageJsonPath !== packageJsonPath) {
    checkCoreVersion(projectPackageJsonPath, expectedVersion);
  }

  return require.resolve(`vite${subpath}`);
}
