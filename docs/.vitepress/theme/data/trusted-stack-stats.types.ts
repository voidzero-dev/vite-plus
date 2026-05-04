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
  projects: TrustedStackStatProject[];
}
