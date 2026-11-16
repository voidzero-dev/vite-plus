import { existsSync, realpathSync } from 'node:fs';
import { basename, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export function resolveWindowsLintBackendExecutable(
  pathCandidates: string[],
  options: {
    exists: (path: string) => boolean;
    getRealpathCandidates?: () => string[];
  },
): string {
  const backendPath = pathCandidates.find((path) => options.exists(path));
  if (backendPath) {
    return backendPath;
  }
  if (options.getRealpathCandidates) {
    try {
      const realPath = options.getRealpathCandidates().find((path) => options.exists(path));
      if (realPath) {
        return realPath;
      }
    } catch {
      // Report the original candidates if Bun's realpath fallback fails.
    }
  }
  throw new Error(
    'Unable to resolve type-aware lint executable, tried:\n' +
      pathCandidates.map((path) => `- ${path}`).join('\n'),
  );
}

export function resolveLintBackendExecutable(backendBinPath: string, scriptUrl: string): string {
  if (process.platform !== 'win32') {
    return backendBinPath;
  }

  // On Windows, try .exe first (bun creates .exe), then .cmd (npm/pnpm/yarn create .cmd)
  const executableName = basename(backendBinPath);
  const scriptDir = dirname(fileURLToPath(scriptUrl));
  const localBinDir = join(scriptDir, '..', 'node_modules', '.bin');
  const backendPackagePath = dirname(dirname(backendBinPath));
  const projectBinDir = join(backendPackagePath, '..', '.bin');
  const pathCandidates = [
    join(localBinDir, `${executableName}.exe`),
    join(localBinDir, `${executableName}.cmd`),
    join(projectBinDir, `${executableName}.exe`),
    join(projectBinDir, `${executableName}.cmd`),
  ];

  return resolveWindowsLintBackendExecutable(pathCandidates, {
    exists: existsSync,
    // Bun stores packages in .bun/ cache dirs where the symlinked paths above won't match.
    getRealpathCandidates: () => {
      const realPkgDir = realpathSync(join(scriptDir, '..'));
      const realBinDir = join(dirname(realPkgDir), '.bin');
      return [join(realBinDir, `${executableName}.exe`), join(realBinDir, `${executableName}.cmd`)];
    },
  });
}
