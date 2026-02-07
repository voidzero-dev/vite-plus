import { execSync } from 'node:child_process';
import { join } from 'node:path';

import { ecosystemCiDir, tgzDir } from './paths.ts';
import repos from './repo.json' with { type: 'json' };

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

async function migrateProject(project: string) {
  const repoRoot = join(ecosystemCiDir, project);
  const repoConfig = repos[project as keyof typeof repos];
  const directory = 'directory' in repoConfig ? repoConfig.directory : undefined;
  const cwd = directory ? join(repoRoot, directory) : repoRoot;
  // run vp migrate
  const cli = process.env.VITE_PLUS_CLI_BIN ?? 'vp';
  execSync(`${cli} migrate --no-agent --no-interactive`, {
    cwd,
    stdio: 'inherit',
    env: {
      ...process.env,
      VITE_PLUS_OVERRIDE_PACKAGES: JSON.stringify({
        vite: `file:${tgzDir}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
        vitest: `file:${tgzDir}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
        '@voidzero-dev/vite-plus-core': `file:${tgzDir}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
        '@voidzero-dev/vite-plus-test': `file:${tgzDir}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
      }),
      VITE_PLUS_VERSION: `file:${tgzDir}/vite-plus-0.0.0.tgz`,
    },
  });
}

await migrateProject(project);
