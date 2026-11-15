import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';

import { VITE_PLUS_CORE_NAME, coreAliasVersion } from './utils/constants.ts';
import { readNearestPackageJson } from './utils/package.ts';

/**
 * Check that a resolved `vite` is the core package, at `expectedVersion` when
 * one is given. Returns the core version.
 */
function checkCoreVersion(
  packageJsonPath: string,
  expectedVersion: string | undefined,
): string | undefined {
  const pkg = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
    name?: string;
    version?: string;
  };
  if (
    pkg.name !== VITE_PLUS_CORE_NAME ||
    (expectedVersion !== undefined && pkg.version !== expectedVersion)
  ) {
    const expected =
      expectedVersion === undefined
        ? VITE_PLUS_CORE_NAME
        : `${VITE_PLUS_CORE_NAME}@${expectedVersion}`;
    throw new Error(
      `Expected ${expected}, but found ${pkg.name}@${pkg.version} at ${packageJsonPath}. ` +
        'Run `vp migrate` to align the Vite alias, then run `vp install`.',
    );
  }
  return pkg.version;
}

/**
 * Read the core version that the selected CLI pins through its `vite` alias.
 * The core version encodes the bundled Vite version, so it differs from the
 * CLI version (rfcs/core-version-alignment.md). Packing turns the repository's
 * `workspace:` spec into the exact alias. Any other spec pins no version.
 */
function readAliasCoreVersion(cliPackageJsonPath: string): string | undefined {
  const { dependencies } = JSON.parse(readFileSync(cliPackageJsonPath, 'utf8')) as {
    dependencies?: Record<string, string>;
  };
  return coreAliasVersion(dependencies?.vite);
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
  let corePackageJsonPath: string;
  try {
    corePackageJsonPath = cliRequire.resolve('vite/package.json');
  } catch (cause) {
    throw new Error('Could not resolve the bundled Vite dependency. Run `vp install`.', { cause });
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
