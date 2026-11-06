import { existsSync, realpathSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export function resolveWindowsLintBackendExecutable(
  pathCandidates: string[],
  options: {
    exists: (path: string) => boolean;
    getRealpathCandidates?: () => string[];
  },
): string {
  let backendPath = pathCandidates.find((p) => options.exists(p)) ?? '';
  if (!backendPath && options.getRealpathCandidates) {
    try {
      backendPath = options.getRealpathCandidates().find((p) => options.exists(p)) ?? '';
    } catch {
      // realpath failed, fall through to default
    }
  }
  if (!backendPath) {
    throw new Error(
      'Unable to resolve type-aware lint executable, tried:\n' +
        pathCandidates.map((path) => `- ${path}`).join('\n'),
    );
  }
  return backendPath;
}

export function resolveLintBackendExecutable(
  backendBinPath: string,
  scriptUrl: string,
  executableName: string,
): string {
  if (process.platform !== 'win32') {
    return backendBinPath;
  }

  // On Windows, try .exe first (bun creates .exe), then .cmd (npm/pnpm/yarn create .cmd)
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
