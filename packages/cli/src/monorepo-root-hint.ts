import path from 'node:path';

import type { WorkspaceInfoOptional } from './types/index.ts';

const DEV_ROOT_VALUE_FLAGS = new Set([
  '-c',
  '--config',
  '--host',
  '--port',
  '--strictPort',
  '--base',
  '--mode',
  '--logLevel',
]);

export function devCommandHasExplicitRoot(args: string[]): boolean {
  const rest = args.slice(1);
  let skipNext = false;

  for (const arg of rest) {
    if (skipNext) {
      skipNext = false;
      continue;
    }
    if (arg === '--') {
      break;
    }
    if (DEV_ROOT_VALUE_FLAGS.has(arg)) {
      skipNext = true;
      continue;
    }
    if ([...DEV_ROOT_VALUE_FLAGS].some((flag) => arg.startsWith(`${flag}=`))) {
      continue;
    }
    if (!arg.startsWith('-')) {
      return true;
    }
  }

  return false;
}

export function shouldWarnDevFromMonorepoRoot(
  command: string | undefined,
  args: string[],
  cwd: string,
  workspaceInfo: WorkspaceInfoOptional,
): boolean {
  if (command !== 'dev' || args.includes('--help') || args.includes('-h')) {
    return false;
  }
  if (!workspaceInfo.isMonorepo || devCommandHasExplicitRoot(args)) {
    return false;
  }

  return path.resolve(cwd) === path.resolve(workspaceInfo.rootDir);
}

export function formatDevMonorepoRootHint(workspaceInfo: WorkspaceInfoOptional): string {
  const example =
    workspaceInfo.packages[0]?.path ?? `${workspaceInfo.parentDirs[0] ?? 'apps'}/website`;
  return (
    `Detected a monorepo root. \`vp dev\` starts Vite in the workspace root, which can look ` +
    `successful while serving no app. Run \`vp run dev\` to use package tasks, or run ` +
    `\`vp dev ${example}\` for a specific app.`
  );
}
