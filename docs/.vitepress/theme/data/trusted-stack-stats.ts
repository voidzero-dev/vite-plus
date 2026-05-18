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

export const trustedStackStats = raw as TrustedStackStatsFile;

export function trustedStackById(id: TrustedStackProjectId): TrustedStackStatProject {
  const project = trustedStackStats.projects.find((p) => p.id === id);
  if (!project) {
    throw new Error(`trusted-stack-stats.json: missing project "${id}"`);
  }
  return project;
}
