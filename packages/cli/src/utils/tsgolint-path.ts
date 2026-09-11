import { existsSync, realpathSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

export function resolveWindowsTsgolintExecutable(
  pathCandidates: string[],
  options: {
    exists: (path: string) => boolean;
    getRealpathCandidates?: () => string[];
  },
): string {
  let oxlintTsgolintPath = pathCandidates.find((p) => options.exists(p)) ?? '';
  if (!oxlintTsgolintPath && options.getRealpathCandidates) {
    try {
      oxlintTsgolintPath = options.getRealpathCandidates().find((p) => options.exists(p)) ?? '';
    } catch {
      // realpath failed, fall through to default
    }
  }
  if (!oxlintTsgolintPath) {
    throw new Error(
      'Unable to resolve oxlint-tsgolint executable, tried:\n' +
        pathCandidates.map((path) => `- ${path}`).join('\n'),
    );
  }
  return oxlintTsgolintPath;
}

export function resolveTsgolintExecutable(tsgolintBinPath: string, scriptUrl: string): string {
  if (process.platform !== 'win32') {
    return tsgolintBinPath;
  }

  // On Windows, try .exe first (bun creates .exe), then .cmd (npm/pnpm/yarn create .cmd)
  const scriptDir = dirname(fileURLToPath(scriptUrl));
  const localBinDir = join(scriptDir, '..', 'node_modules', '.bin');
  const oxlintTsgolintPackagePath = dirname(dirname(tsgolintBinPath));
  const projectBinDir = join(oxlintTsgolintPackagePath, '..', '.bin');
  const pathCandidates = [
    join(localBinDir, 'tsgolint.exe'),
    join(localBinDir, 'tsgolint.cmd'),
    join(projectBinDir, 'tsgolint.exe'),
    join(projectBinDir, 'tsgolint.cmd'),
  ];

  return resolveWindowsTsgolintExecutable(pathCandidates, {
    exists: existsSync,
    // Bun stores packages in .bun/ cache dirs where the symlinked paths above won't match.
    getRealpathCandidates: () => {
      const realPkgDir = realpathSync(join(scriptDir, '..'));
      const realBinDir = join(dirname(realPkgDir), '.bin');
      return [join(realBinDir, 'tsgolint.exe'), join(realBinDir, 'tsgolint.cmd')];
    },
  });
}
