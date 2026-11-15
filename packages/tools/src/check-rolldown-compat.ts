import { readFileSync } from 'node:fs';
import { createRequire } from 'node:module';
import { join } from 'node:path';

import { findMinimumForRange, satisfies, tryParse, tryParseRange } from 'verkit';

type PackageJson = {
  version?: unknown;
  dependencies?: Record<string, unknown>;
};

function readPackage(file: string): PackageJson {
  return JSON.parse(readFileSync(file, 'utf8'));
}

export function checkRolldownCompatibility(root: string): void {
  const rolldown = readPackage(join(root, 'rolldown/packages/rolldown/package.json'));
  const versionString = typeof rolldown.version === 'string' ? rolldown.version : '';
  const version = tryParse(versionString);
  if (!version) {
    throw new Error('Missing or invalid version in the pinned Rolldown package');
  }

  const coreRequire = createRequire(join(root, 'packages/core/package.json'));
  const consumers = [
    ['vite', join(root, 'vite/packages/vite/package.json')],
    ['tsdown', coreRequire.resolve('tsdown/package.json')],
  ] as const;

  for (const [name, file] of consumers) {
    // Read upstream declarations: the workspace override can make incompatible
    // requirements resolve to the same installed Rolldown package.
    const pkg = readPackage(file);
    const declaredRange =
      typeof pkg.dependencies?.rolldown === 'string' ? pkg.dependencies.rolldown : '';
    const range = declaredRange.trim() !== '' ? tryParseRange(declaredRange) : null;
    const minimum = range && findMinimumForRange(range);
    if (!range || !minimum) {
      throw new Error(`Missing or invalid dependencies.rolldown in ${file}`);
    }

    // A caret range can accept a later minor. Keep the release line from the
    // range's minimum version as well as checking the complete range.
    if (version.major !== minimum.major || version.minor !== minimum.minor) {
      throw new Error(
        `Rolldown ${versionString} must use ${name}'s ${minimum.major}.${minimum.minor}.x release line (${declaredRange})`,
      );
    }
    if (!satisfies(version, range)) {
      throw new Error(
        `Rolldown ${versionString} does not satisfy ${name}'s dependencies.rolldown (${declaredRange})`,
      );
    }
  }
}

if (import.meta.main) {
  try {
    checkRolldownCompatibility(process.cwd());
    console.log('Rolldown is compatible with Vite and tsdown.');
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
