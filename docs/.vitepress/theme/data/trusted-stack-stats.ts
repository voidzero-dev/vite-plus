import raw from './trusted-stack-stats.json';

export type TrustedStackProjectId = 'vite' | 'vitest' | 'oxc';

export interface TrustedStackStatProject {
  id: TrustedStackProjectId;
  npmPackage: string;
  githubRepo: string;
  npmWeeklyDownloads: number;
  githubStargazers: number;
  npmWeeklyDownloadsDisplay: string;
  githubStarsDisplay: string;
}

export interface TrustedStackStatsFile {
  generatedAt: string;
  projects: TrustedStackStatProject[];
}

export const trustedStackStats = raw as TrustedStackStatsFile;

export function trustedStackById(id: TrustedStackProjectId): TrustedStackStatProject {
  const project = trustedStackStats.projects.find((p) => p.id === id);
  if (!project) {
    throw new Error(`trusted-stack-stats.json: missing project "${id}"`);
  }
  return project;
}
