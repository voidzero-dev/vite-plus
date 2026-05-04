/**
 * Fetches last-week npm download counts and GitHub star counts, then writes
 * docs/.vitepress/theme/data/trusted-stack-stats.json for the docs home page.
 *
 * Run from repo root: `pnpm -C docs update-trusted-stack-stats`
 * Or: `node docs/scripts/fetch-trusted-stack-stats.mjs`
 */
import { writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const OUT = join(__dirname, '../.vitepress/theme/data/trusted-stack-stats.json');

const PROJECTS = [
  { id: 'vite', npmPackage: 'vite', githubRepo: 'vitejs/vite' },
  { id: 'vitest', npmPackage: 'vitest', githubRepo: 'vitest-dev/vitest' },
  /** OXC row uses `oxlint` npm weekly downloads as a concrete proxy for the Oxc toolchain. */
  { id: 'oxc', npmPackage: 'oxlint', githubRepo: 'oxc-project/oxc' },
];

/**
 * @param {number} n
 * @returns {string}
 */
function formatWeeklyDownloads(n) {
  if (n >= 10_000_000) {
    return `${Math.round(n / 1e6)}m+`;
  }
  const m = n / 1e6;
  const s = m.toFixed(1).replace(/\.0$/, '');
  return `${s}m+`;
}

/**
 * @param {number} s
 * @returns {string}
 */
function formatStars(s) {
  return `${(s / 1000).toFixed(1)}k`;
}

/**
 * @param {string} pkg
 * @returns {Promise<number>}
 */
async function npmLastWeekDownloads(pkg) {
  const url = `https://api.npmjs.org/downloads/point/last-week/${encodeURIComponent(pkg)}`;
  const res = await fetch(url);
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`npm API ${pkg}: HTTP ${res.status} ${body}`);
  }
  const data = await res.json();
  if (typeof data.downloads !== 'number') {
    throw new Error(`npm API ${pkg}: unexpected payload`);
  }
  return data.downloads;
}

/**
 * @param {string} repo "owner/name"
 * @returns {Promise<number>}
 */
async function fetchGithubStargazers(repo) {
  const url = `https://api.github.com/repos/${repo}`;
  /** @type {Record<string, string>} */
  const headers = {
    Accept: 'application/vnd.github+json',
    'X-GitHub-Api-Version': '2022-11-28',
    'User-Agent': 'voidzero-dev/vite-plus (docs/scripts/fetch-trusted-stack-stats.mjs)',
  };
  const token = process.env.GITHUB_TOKEN;
  if (token) {
    headers.Authorization = `Bearer ${token}`;
  }
  const res = await fetch(url, { headers });
  if (!res.ok) {
    const body = await res.text();
    throw new Error(`GitHub API ${repo}: HTTP ${res.status} ${body}`);
  }
  const data = await res.json();
  if (typeof data.stargazers_count !== 'number') {
    throw new Error(`GitHub API ${repo}: unexpected payload`);
  }
  return data.stargazers_count;
}

async function main() {
  const projects = [];
  for (const p of PROJECTS) {
    const [npmWeeklyDownloads, stars] = await Promise.all([
      npmLastWeekDownloads(p.npmPackage),
      fetchGithubStargazers(p.githubRepo),
    ]);
    projects.push({
      id: p.id,
      npmPackage: p.npmPackage,
      githubRepo: p.githubRepo,
      npmWeeklyDownloads,
      githubStargazers: stars,
      npmWeeklyDownloadsDisplay: formatWeeklyDownloads(npmWeeklyDownloads),
      githubStarsDisplay: formatStars(stars),
    });
  }
  const payload = {
    generatedAt: new Date().toISOString(),
    projects,
  };
  await writeFile(OUT, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  console.log(`Wrote ${OUT}`);
}

main().catch((err) => {
  console.error(err);
  process.exitCode = 1;
});
