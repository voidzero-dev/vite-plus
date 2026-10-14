import fs from 'node:fs';
import path from 'node:path';

/**
 * Find the workspace root by walking up from `startDir` looking for
 * monorepo indicators (pnpm-workspace.yaml, workspaces in package.json, lerna.json).
 */
export function findWorkspaceRoot(startDir: string): string | undefined {
  let dir = path.resolve(startDir);
  while (true) {
    if (fs.existsSync(path.join(dir, 'pnpm-workspace.yaml'))) {
      return dir;
    }
    const pkgPath = path.join(dir, 'package.json');
    if (fs.existsSync(pkgPath)) {
      try {
        const pkg = JSON.parse(fs.readFileSync(pkgPath, 'utf-8'));
        if (pkg.workspaces) {
          return dir;
        }
      } catch {
        // Skip malformed package.json and continue searching parent directories
      }
    }
    if (fs.existsSync(path.join(dir, 'lerna.json'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  return undefined;
}

export function* dirsUpTo(from: string, to?: string, pathImpl: path.PlatformPath = path) {
  for (let dir = from; ; dir = pathImpl.dirname(dir)) {
    yield dir;
    if (dir === to || dir === pathImpl.dirname(dir)) {
      return;
    }
  }
}

export function findPackageRoot(startDir: string): string | undefined {
  for (const dir of dirsUpTo(path.resolve(startDir))) {
    if (fs.existsSync(path.join(dir, 'package.json'))) {
      return dir;
    }
  }
  return undefined;
}
