import { writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

/**
 * Fetches last-week npm download counts and GitHub star counts, then writes
 * `trusted-stack-stats.json` for the docs home page.
 *
 * Requires Node.js >=22.18 (strip types). Run:
 *   `pnpm -C docs update-trusted-stack-stats`
 * or: `node docs/.vitepress/theme/data/fetch-trusted-stack-stats.ts`
 */
import type {
  TrustedStackProjectId,
  TrustedStackStatProject,
  TrustedStackStatsFile,
} from './trusted-stack-stats.types';

const currentDir = dirname(fileURLToPath(import.meta.url));
const OUT = join(currentDir, 'trusted-stack-stats.json');

interface ProjectSource {
  readonly id: TrustedStackProjectId;
  readonly npmPackage: string;
  readonly githubRepo: string;
}

const PROJECTS: readonly ProjectSource[] = [
  { id: 'vite', npmPackage: 'vite', githubRepo: 'vitejs/vite' },
  { id: 'vitest', npmPackage: 'vitest', githubRepo: 'vitest-dev/vitest' },
  /** OXC row uses `oxlint` npm weekly downloads as a concrete proxy for the Oxc toolchain. */
  { id: 'oxc', npmPackage: 'oxlint', githubRepo: 'oxc-project/oxc' },
];

function formatWeeklyDownloads(n: number): string {
  if (n >= 10_000_000) {
    // "m+" reads as a lower bound, so avoid rounding up.
    return `${Math.floor(n / 1e6)}m+`;
  }
  const m = n / 1e6;
  const s = m.toFixed(1).replace(/\.0$/, '');
  return `${s}m+`;
}

function formatStars(s: number): string {
  return `${(s / 1000).toFixed(1)}k`;
}

function parseNpmDownloadsJson(data: unknown, pkg: string): number {
  if (typeof data !== 'object' || data === null || !('downloads' in data)) {
    throw new Error(`npm API ${pkg}: unexpected payload`);
  }
  const downloads = (data as { downloads: unknown }).downloads;
  if (typeof downloads !== 'number') {
    throw new Error(`npm API ${pkg}: unexpected payload`);
  }
  return downloads;
}

async function npmLastWeekDownloads(pkg: string): Promise<number> {
  const url = `https://api.npmjs.org/downloads/point/last-week/${encodeURIComponent(pkg)}`;
  const res = await fetch(url);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`npm API ${pkg}: HTTP ${res.status} ${body}`);
  }
  return parseNpmDownloadsJson(await res.json(), pkg);
}

function parseGithubRepoJson(data: unknown, repo: string): number {
  if (typeof data !== 'object' || data === null || !('stargazers_count' in data)) {
    throw new Error(`GitHub API ${repo}: unexpected payload`);
  }
  const count = (data as { stargazers_count: unknown }).stargazers_count;
  if (typeof count !== 'number') {
    throw new Error(`GitHub API ${repo}: unexpected payload`);
  }
  return count;
}

async function fetchGithubStargazers(repo: string): Promise<number> {
  const url = `https://api.github.com/repos/${repo}`;
  const headers: Record<string, string> = {
    Accept: 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
    'User-Agent':
      'voidzero-dev/vite-plus (docs/.vitepress/theme/data/fetch-trusted-stack-stats.ts)',
  };
  const token = process.env.GITHUB_TOKEN;
  if (token !== undefined && token !== '') {
    headers.Authorization = `Bearer ${token}`;
  }
  const res = await fetch(url, { headers });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`GitHub API ${repo}: HTTP ${res.status} ${body}`);
  }
  return parseGithubRepoJson(await res.json(), repo);
}

async function main(): Promise<void> {
  const projects: TrustedStackStatProject[] = [];
  for (const p of PROJECTS) {
    const [npmWeeklyDownloads, stars] = await Promise.all([
      npmLastWeekDownloads(p.npmPackage),
      fetchGithubStargazers(p.githubRepo),
    ]);
    const row: TrustedStackStatProject = {
      id: p.id,
      npmPackage: p.npmPackage,
      githubRepo: p.githubRepo,
      npmWeeklyDownloads,
      githubStargazers: stars,
      npmWeeklyDownloadsDisplay: formatWeeklyDownloads(npmWeeklyDownloads),
      githubStarsDisplay: formatStars(stars),
    };
    projects.push(row);
  }
  const payload: TrustedStackStatsFile = { projects };
  await writeFile(OUT, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.error(`Wrote ${OUT} at ${new Date().toISOString()}`);
}

void main().catch((err: unknown) => {
  console.error(err);
  process.exitCode = 1;
});
