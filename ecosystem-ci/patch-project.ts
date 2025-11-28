import fs from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import vitestPackageJson from '../packages/test/package.json' with { type: 'json' };
import repos from './repo.json' with { type: 'json' };

const projectDir = dirname(fileURLToPath(import.meta.url));

const projects = Object.keys(repos);

const project = process.argv[2];

if (!projects.includes(project)) {
  console.error(`Project ${project} is not defined in repo.json`);
  process.exit(1);
}

const tgzPath = join(projectDir, '..', 'tmp', 'tgz');

async function patchVibeDashboard() {
  const pnpmWorkspacePath = join(projectDir, 'vibe-dashboard', 'pnpm-workspace.yaml');
  const pnpmWorkspaceFile = fs
    .readFileSync(pnpmWorkspacePath, 'utf8')
    .replace(
      '"vite": "npm:@voidzero-dev/vite-plus-core"',
      `"vite": "file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz"`,
    )
    .replace(
      '"vitest": "npm:@voidzero-dev/vite-plus-test"',
      `"vitest": "file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz"
  "@voidzero-dev/vite-plus": "file:${tgzPath}/voidzero-dev-vite-plus-0.0.0.tgz"
  "@voidzero-dev/vite-plus-core": "file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz"
  "@voidzero-dev/vite-plus-test": "file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz"`,
    );
  fs.writeFileSync(pnpmWorkspacePath, pnpmWorkspaceFile);
}

async function patchSkeleton() {
  const packageJsonPath = join(projectDir, 'skeleton', 'package.json');
  const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));

  // Change test command from "vitest run" to "vite test"
  packageJson.scripts.test = 'vite test';

  // Add pnpm overrides with tgz files
  packageJson.pnpm = packageJson.pnpm || {};
  packageJson.pnpm.overrides = {
    ...packageJson.pnpm.overrides,
    vite: `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    'rolldown-vite': `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    vitest: `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
    '@voidzero-dev/vite-plus': `file:${tgzPath}/voidzero-dev-vite-plus-0.0.0.tgz`,
    '@voidzero-dev/vite-plus-core': `file:${tgzPath}/voidzero-dev-vite-plus-core-0.0.0.tgz`,
    '@voidzero-dev/vite-plus-test': `file:${tgzPath}/voidzero-dev-vite-plus-test-0.0.0.tgz`,
  };

  packageJson.devDependencies = {
    ...packageJson.devDependencies,
    '@voidzero-dev/vite-plus': `latest`,
    playwright: `catalog:`,
  };

  fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');

  const vitestVersion = vitestPackageJson.dependencies['@vitest/expect'];

  // Patch pnpm-workspace.yaml
  const pnpmWorkspacePath = join(projectDir, 'skeleton', 'pnpm-workspace.yaml');
  const pnpmWorkspaceContent = fs
    .readFileSync(pnpmWorkspacePath, 'utf8')
    .replace(`trustPolicy: no-downgrade`, '\n')
    .replace(
      /'@vitest\/browser-playwright': [\d.]+/,
      `'@vitest/browser-playwright': ${vitestVersion}`,
    );
  const appendContent = `
minimumReleaseAgeExclude:
  - '@voidzero-dev/*'
  - '@vitest/*'
  - oxlint
  - oxfmt
  - oxlint-tsgolint

peerDependencyRules:
  allowAny:
    - vite
    - vitest
`;
  fs.writeFileSync(pnpmWorkspacePath, pnpmWorkspaceContent + appendContent);
}

switch (project) {
  case 'vibe-dashboard':
    await patchVibeDashboard();
    break;
  case 'skeleton':
    await patchSkeleton();
    break;
  default:
    console.error(`Project ${project} is not supported`);
    process.exit(1);
}
