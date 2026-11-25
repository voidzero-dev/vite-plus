import raw from './trusted-stack-stats.json';
import type {
  TrustedStackProjectId,
  TrustedStackStatProject,
  TrustedStackStatsFile,
} from './trusted-stack-stats.types';

export type {
  TrustedStackProjectId,
  TrustedStackStatProject,
  TrustedStackStatsFile,
} from './trusted-stack-stats.types';

function isTrustedStackProject(
  project: (typeof raw.projects)[number],
): project is TrustedStackStatProject {
  return project.id === 'vite' || project.id === 'vitest' || project.id === 'oxc';
}

const { projects } = raw;
if (!projects.every(isTrustedStackProject)) {
  throw new Error('Unexpected project ID in trusted-stack-stats.json');
}
export const trustedStackStats: TrustedStackStatsFile = { projects };

export function trustedStackById(id: TrustedStackProjectId): TrustedStackStatProject {
  const project = trustedStackStats.projects.find((p) => p.id === id);
  if (!project) {
    throw new Error(`trusted-stack-stats.json: missing project "${id}"`);
  }
  return project;
}
