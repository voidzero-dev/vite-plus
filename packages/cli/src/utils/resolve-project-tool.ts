import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';

import { resolveBundled } from './constants.ts';
import { dirsUpTo, findPackageRoot, findWorkspaceRoot } from './workspace-root.ts';

const require = createRequire(import.meta.url);

const DEPENDENCY_FIELDS = [
  'dependencies',
  'devDependencies',
  'optionalDependencies',
  'peerDependencies',
] as const;

function packageNameOf(specifier: string): string {
  const segments = specifier.split('/');
  return specifier.startsWith('@') ? segments.slice(0, 2).join('/') : segments[0];
}

function declaresDependency(dir: string, packageName: string): boolean {
  try {
    const pkg = JSON.parse(fs.readFileSync(path.join(dir, 'package.json'), 'utf8')) as Record<
      string,
      Record<string, string> | undefined
    >;
    return DEPENDENCY_FIELDS.some((field) => Object.hasOwn(pkg?.[field] ?? {}, packageName));
  } catch {
    return false;
  }
}

function resolveInstalled(
  specifier: string,
  packageName: string,
  owner: string,
  workspaceRoot: string,
): string | undefined {
  if (process.versions.pnp) {
    try {
      return require.resolve(specifier, { paths: [owner] });
    } catch {
      return undefined;
    }
  }
  for (const dir of dirsUpTo(owner, workspaceRoot)) {
    if (fs.existsSync(path.join(dir, 'node_modules', packageName))) {
      return require.resolve(specifier, { paths: [dir] });
    }
  }
  return undefined;
}

export function resolveProjectTool(specifier: string, cwd = process.cwd()): string {
  const packageName = packageNameOf(specifier);
  const packageRoot = findPackageRoot(cwd);
  if (packageRoot) {
    const workspaceRoot = findWorkspaceRoot(packageRoot) ?? packageRoot;
    const owner = [packageRoot, workspaceRoot].find((dir) => declaresDependency(dir, packageName));
    const resolved = owner && resolveInstalled(specifier, packageName, owner, workspaceRoot);
    if (resolved) {
      return resolved;
    }
  }
  return resolveBundled(specifier);
}
