import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';

import cliPkg from '../package.json' with { type: 'json' };

const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';

function checkCoreVersion(packageJsonPath: string): void {
  const pkg = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
    name?: string;
    version?: string;
  };
  if (pkg.name !== CORE_PACKAGE_NAME || pkg.version !== cliPkg.version) {
    throw new Error(
      `Expected ${CORE_PACKAGE_NAME}@${cliPkg.version}, but found ${pkg.name}@${pkg.version} at ${packageJsonPath}. ` +
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
  let packageJsonPath: string;
  try {
    packageJsonPath = require.resolve('vite/package.json');
  } catch (cause) {
    throw new Error('Could not resolve the bundled Vite dependency. Run `vp install`.', { cause });
  }
  checkCoreVersion(packageJsonPath);

  // A migrated project's alias must match the CLI release. Without a project
  // alias, the required dependency above still supplies the bundled commands.
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
    checkCoreVersion(projectPackageJsonPath);
  }

  return require.resolve(`vite${subpath}`);
}
