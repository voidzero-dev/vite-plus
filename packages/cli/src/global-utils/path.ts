import path from 'node:path';

// Get the package root directory (packages/cli)
// Bundled files are in dist/global/, so we walk up until we find package.json
function findPkgRoot(): string {
  let dir = import.meta.dirname;
  while (dir !== path.dirname(dir)) {
    if (path.basename(dir) !== 'dist' && path.basename(dir) !== 'global') {
      return dir;
    }
    dir = path.dirname(dir);
  }
  return dir;
}

export const pkgRoot = findPkgRoot();

export const templatesDir = path.join(pkgRoot, 'templates');
export const rulesDir = path.join(pkgRoot, 'rules');

export function displayRelative(to: string, from = process.cwd()): string {
  return path.relative(from, to).replaceAll('\\', '/');
}
