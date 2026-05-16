import { execSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';

import { ecosystemCiDir, tgzDir } from './paths.ts';
import repos from './repo.json' with { type: 'json' };

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const repoRoot = join(ecosystemCiDir, project);
const repoConfig = repos[project as keyof typeof repos];
const directory = 'directory' in repoConfig ? repoConfig.directory : undefined;
const cwd = directory ? join(repoRoot, directory) : repoRoot;
// run vp migrate
const cli = process.env.VP_CLI_BIN ?? 'vp';

if (project === 'rollipop') {
  const oxfmtrc = await readFile(join(repoRoot, '.oxfmtrc.json'), 'utf-8');
  await writeFile(
    join(repoRoot, '.oxfmtrc.json'),
    oxfmtrc.replace('      ["ts-equals-import"],\n', ''),
    'utf-8',
  );
}

if (project === 'vinext') {
  // vinext sets `minimumReleaseAge` (24h) which blocks fresh upstream upgrades
  // (e.g. oxc 0.129.0 published <24h ago). Disable it for the ecosystem run so
  // upgrade-deps PRs can install transitive deps that were just published.
  const workspacePath = join(repoRoot, 'pnpm-workspace.yaml');
  const workspace = await readFile(workspacePath, 'utf-8');
  const patched = workspace.replace(/^minimumReleaseAge:.*$/m, 'minimumReleaseAge: 0');
  if (patched === workspace) {
    throw new Error(`vinext patch: \`minimumReleaseAge:\` not found in ${workspacePath}`);
  }
  await writeFile(workspacePath, patched, 'utf-8');
}

// Projects that already use vite-plus need VP_FORCE_MIGRATE=1 so
// vp migrate runs full dependency rewriting instead of skipping.
const forceFreshMigration = 'forceFreshMigration' in repoConfig && repoConfig.forceFreshMigration;

// Bun is uniquely strict about vitest's `peer vite ^6 || ^7 || ^8` resolution
// (https://github.com/oven-sh/bun/issues/8406): it checks both the override
// target's package name and version. Point bun-based projects at the
// vite-7.99.0 alias tgz (a copy of core renamed to "vite" with a satisfying
// version); pnpm/npm/yarn must keep pointing at the real core tgz, otherwise
// they trip a registry lookup for "vite@<version>" when a workspace
// sub-package and the override both reference the same vite-named alias.
const isBunProject = project === 'bun-vite-template';
const viteOverrideTgz = isBunProject ? `vite-7.99.0.tgz` : `voidzero-dev-vite-plus-core-0.0.0.tgz`;

execSync(`${cli} migrate --no-agent --no-interactive`, {
  cwd,
  stdio: 'inherit',
  env: {
    ...process.env,
    ...(forceFreshMigration ? { VP_FORCE_MIGRATE: '1' } : {}),
    VP_OVERRIDE_PACKAGES: JSON.stringify({
      vite: `file:${tgzDir}/${viteOverrideTgz}`,
      '@voidzero-dev/vite-plus-core': `file:${tgzDir}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    }),
    VP_VERSION: `file:${tgzDir}/vite-plus-0.0.0.tgz`,
  },
});
