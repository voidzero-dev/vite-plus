import { createRequire } from 'node:module';
import path from 'node:path';

import { resolveBundled } from './constants.ts';
import { findWorkspaceRoot, readNearestPackageJson } from './package.ts';

function declaresDependency(dir: string, packageName: string): boolean {
  const pkg = readNearestPackageJson(dir);
  return ['dependencies', 'devDependencies', 'optionalDependencies', 'peerDependencies'].some(
    (field) => Object.hasOwn(pkg?.[field] ?? {}, packageName),
  );
}

export function resolveProjectTool(specifier: string, cwd = process.cwd()): string {
  const packageName = specifier.split('/')[0];
  const owner = [cwd, findWorkspaceRoot(cwd)].find(
    (dir) => dir && declaresDependency(dir, packageName),
  );
  if (owner) {
    try {
      return createRequire(path.join(owner, 'package.json')).resolve(specifier);
    } catch (cause) {
      if ((cause as NodeJS.ErrnoException).code !== 'MODULE_NOT_FOUND') {
        throw cause;
      }
    }
  }
  return resolveBundled(specifier);
}
