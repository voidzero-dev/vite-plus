import { createRequire } from 'node:module';
import { join } from 'node:path';

import { VITE_PLUS_CORE_NAME, coreAliasVersion } from './utils/constants.ts';
import { errorCode } from './utils/error.ts';
import { isObjectValue } from './utils/json-value.ts';
import { readJsonFile } from './utils/json.ts';
import { readNearestPackageJson } from './utils/package.ts';

/**
 * Check that a resolved `vite` is the core package, at `expectedVersion` when
 * one is given. Returns the core version.
 */
function checkCoreVersion(
  packageJsonPath: string,
  expectedVersion: string | undefined,
): string | undefined {
  const pkg = readJsonFile(packageJsonPath);
  if (
    pkg.name !== VITE_PLUS_CORE_NAME ||
    (expectedVersion !== undefined && pkg.version !== expectedVersion)
  ) {
    const expected =
      expectedVersion === undefined
        ? VITE_PLUS_CORE_NAME
        : `${VITE_PLUS_CORE_NAME}@${expectedVersion}`;
    throw new Error(
      `Expected ${expected}, but found ${typeof pkg.name === 'string' ? pkg.name : JSON.stringify(pkg.name)}@${typeof pkg.version === 'string' ? pkg.version : JSON.stringify(pkg.version)} at ${packageJsonPath}. ` +
        'Run `vp migrate` to align the Vite alias, then run `vp install`.',
    );
  }
  return typeof pkg.version === 'string' ? pkg.version : undefined;
}

/**
 * Read the core version that the selected CLI pins through its `vite` alias.
 * The core version encodes the bundled Vite version, so it differs from the
 * CLI version (rfcs/core-version-alignment.md). Packing turns the repository's
 * `workspace:` spec into the exact alias. Any other spec pins no version.
 */
function readAliasCoreVersion(cliPackageJsonPath: string): string | undefined {
  const { dependencies } = readJsonFile(cliPackageJsonPath);
  return coreAliasVersion(
    isObjectValue(dependencies) && typeof dependencies.vite === 'string'
      ? dependencies.vite
      : undefined,
  );
}

function checkProjectCoreVersion(
  cwd: string,
  corePackageJsonPath: string,
  expectedVersion: string | undefined,
): void {
  // npm can hoist an upstream Vite peer even when the project only declares
  // vite-plus. Validate Vite only when the nearest package declares it.
  const projectPackage = readNearestPackageJson(cwd);
  const declaresVite = [
    'dependencies',
    'devDependencies',
    'optionalDependencies',
    'peerDependencies',
  ].some((field) => {
    const dependencies = projectPackage?.[field];
    return isObjectValue(dependencies) && Object.hasOwn(dependencies, 'vite');
  });
  if (!declaresVite) {
    return;
  }

  const projectRequire = createRequire(join(cwd, 'package.json'));
  let projectPackageJsonPath: string | undefined;
  try {
    projectPackageJsonPath = projectRequire.resolve('vite/package.json');
  } catch (error) {
    if (errorCode(error) !== 'MODULE_NOT_FOUND') {
      throw error;
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
  let corePackageJsonPath: string;
  try {
    corePackageJsonPath = cliRequire.resolve('vite/package.json');
  } catch (error) {
    throw new Error('Could not resolve the bundled Vite dependency. Run `vp install`.', {
      cause: error,
    });
  }
  // Read the selected CLI's installed manifest. A static JSON import is inlined
  // during the build, before CI and preview packers can stamp a new version.
  // The project's core must match the core that the CLI resolves, so a stale
  // project core also fails when the CLI alias pins no version.
  const coreVersion = checkCoreVersion(
    corePackageJsonPath,
    readAliasCoreVersion(cliRequire.resolve('vite-plus/package.json')),
  );
  checkProjectCoreVersion(cwd, corePackageJsonPath, coreVersion);

  return cliRequire.resolve(`vite${subpath}`);
}
