import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';

import { readNearestPackageJson } from './utils/package.ts';

const CORE_PACKAGE_NAME = '@voidzero-dev/vite-plus-core';
const CORE_ALIAS_PREFIX = `npm:${CORE_PACKAGE_NAME}@`;

function checkCoreVersion(packageJsonPath: string, expectedVersion: string | undefined): void {
  const pkg = JSON.parse(readFileSync(packageJsonPath, 'utf8')) as {
    name?: string;
    version?: string;
  };
  if (
    pkg.name !== CORE_PACKAGE_NAME ||
    (expectedVersion !== undefined && pkg.version !== expectedVersion)
  ) {
    const expected =
      expectedVersion === undefined ? CORE_PACKAGE_NAME : `${CORE_PACKAGE_NAME}@${expectedVersion}`;
    throw new Error(
      `Expected ${expected}, but found ${pkg.name}@${pkg.version} at ${packageJsonPath}. ` +
        'Run `vp migrate` to align the Vite alias, then run `vp install`.',
    );
  }
}

/**
 * Read the core version that the selected CLI pins through its `vite` alias.
 * The core version encodes the bundled Vite version, so it differs from the
 * CLI version (rfcs/core-version-alignment.md). Packing turns the repository's
 * `workspace:` spec into the exact alias. A spec without an exact version
 * falls back to the version of the core that the CLI resolves, so a stale
 * project core still fails in the repository layout.
 */
function readExpectedCoreVersion(
  cliPackageJsonPath: string,
  corePackageJsonPath: string,
): string | undefined {
  const { dependencies } = JSON.parse(readFileSync(cliPackageJsonPath, 'utf8')) as {
    dependencies?: Record<string, string>;
  };
  const spec = dependencies?.vite;
  if (spec?.startsWith(CORE_ALIAS_PREFIX)) {
    return spec.slice(CORE_ALIAS_PREFIX.length);
  }
  const core = JSON.parse(readFileSync(corePackageJsonPath, 'utf8')) as {
    name?: string;
    version?: string;
  };
  return core.name === CORE_PACKAGE_NAME ? core.version : undefined;
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
  const expectedVersion = readExpectedCoreVersion(
    cliRequire.resolve('vite-plus/package.json'),
    corePackageJsonPath,
  );
  checkCoreVersion(corePackageJsonPath, expectedVersion);
  checkProjectCoreVersion(cwd, corePackageJsonPath, expectedVersion);

  return cliRequire.resolve(`vite${subpath}`);
}
