import fs from 'node:fs';
import { createRequire } from 'node:module';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const BOOTSTRAP_ACTIVE_ENV = 'VITE_PLUS_DELEGATE_BOOTSTRAP_ACTIVE';

function normalizeFilePath(filePath: string) {
  return path.resolve(filePath);
}

export function resolveLocalVitePlusBin(projectCwd: string, globalBinPath: string): string | null {
  if (process.env[BOOTSTRAP_ACTIVE_ENV] === '1') {
    return null;
  }

  try {
    const require = createRequire(path.join(projectCwd, 'noop.js'));
    const packageJsonPath = require.resolve('vite-plus/package.json');
    const packageDir = path.dirname(packageJsonPath);
    const localBinPath = path.join(packageDir, 'dist', 'bin.js');

    if (!fs.existsSync(localBinPath)) {
      return null;
    }

    if (normalizeFilePath(localBinPath) === normalizeFilePath(globalBinPath)) {
      return null;
    }

    return localBinPath;
  } catch {
    return null;
  }
}

export function getDelegatedBinPath(projectCwd: string, globalBinPath: string) {
  return resolveLocalVitePlusBin(projectCwd, globalBinPath) ?? globalBinPath;
}

async function main() {
  const [globalBinPath, ...args] = process.argv.slice(2);
  if (!globalBinPath) {
    throw new Error('Missing global vite-plus entry point');
  }

  const delegatedBinPath = getDelegatedBinPath(process.cwd(), globalBinPath);

  process.env[BOOTSTRAP_ACTIVE_ENV] = '1';
  process.argv = [process.execPath, delegatedBinPath, ...args];

  await import(pathToFileURL(delegatedBinPath).href);
}

if (process.argv[1] && normalizeFilePath(process.argv[1]) === fileURLToPath(import.meta.url)) {
  void main().catch((err: unknown) => {
    const message = err instanceof Error ? err.message : String(err);
    console.error(message);
    process.exit(1);
  });
}
