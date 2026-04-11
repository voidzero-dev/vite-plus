import fs from 'node:fs';
import path from 'node:path';

// Get the package root directory (packages/cli)
// Works from both source (src/utils/) and bundled (dist/) locations
function findPkgRoot(): string {
  let dir = import.meta.dirname;
  while (dir !== path.dirname(dir)) {
    if (fs.existsSync(path.join(dir, 'package.json'))) {
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
