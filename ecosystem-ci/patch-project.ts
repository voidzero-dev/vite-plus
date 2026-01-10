import { execSync } from 'node:child_process';
import { appendFileSync, existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import repos from './repo.json' with { type: 'json' };

const projectDir = dirname(fileURLToPath(import.meta.url));

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const tgzPath = join(projectDir, '..', 'tmp', 'tgz');

/**
 * Add public-hoist-pattern to .npmrc for packages that need to be accessible
 * from createRequire().resolve in other packages.
 *
 * This is needed because plugin-react-swc uses createRequire(import.meta.url).resolve
 * to resolve SWC plugins like @swc/plugin-emotion, which are only installed as
 * devDependencies of the playground packages.
 */
function ensurePublicHoist(repoRoot: string) {
  const npmrcPath = join(repoRoot, '.npmrc');
  const patterns = ['@swc/*'];

  for (const pattern of patterns) {
    const line = `public-hoist-pattern[]=${pattern}`;
    if (existsSync(npmrcPath)) {
      let content = readFileSync(npmrcPath, 'utf-8');
      if (!content.includes(line)) {
        // Insert at the beginning of the file to ensure it's before any lines
        // that might have issues (like ${GITHUB_TOKEN} which causes parsing warnings)
        content = `${line}\n${content}`;
        writeFileSync(npmrcPath, content);
        console.log(`Added ${line} to .npmrc`);
      }
    } else {
      appendFileSync(npmrcPath, `${line}\n`);
      console.log(`Created .npmrc with ${line}`);
    }
  }
}

async function migrateProject(project: string) {
  const repoRoot = join(projectDir, project);

  // Ensure packages needed by plugin-react-swc are publicly hoisted
  if (project === 'vite-plugin-react') {
    ensurePublicHoist(repoRoot);
  }

  // run vite migrate
  execSync('vite migrate', {
    cwd: repoRoot,
    stdio: 'inherit',
    env: {
      ...process.env,
      VITE_PLUS_OVERRIDE_PACKAGES: JSON.stringify({
        vite: `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
        vitest: `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
        '@voidzero-dev/vite-plus-core': `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
        '@voidzero-dev/vite-plus-test': `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
      }),
      VITE_PLUS_VERSION: `file:${tgzPath}/voidzero-dev-vite-plus-0.0.0.tgz`,
    },
  });
}

await migrateProject(project);
