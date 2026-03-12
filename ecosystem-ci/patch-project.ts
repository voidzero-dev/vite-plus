import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
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
const cli = process.env.VITE_PLUS_CLI_BIN ?? 'vp';

if (project === 'rollipop') {
  const oxfmtrc = await readFile(join(repoRoot, '.oxfmtrc.json'), 'utf-8');
  await writeFile(
    join(repoRoot, '.oxfmtrc.json'),
    oxfmtrc.replace('      ["ts-equals-import"],\n', ''),
    'utf-8',
  );
}

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

// Enable cacheScripts so e2e tests exercise the cache hit/miss paths.
// Migration may create vite.config.ts, preserve an existing .ts/.js, or create none at all.
const tsPath = join(cwd, 'vite.config.ts');
const jsPath = join(cwd, 'vite.config.js');
let viteConfigPath: string;
if (existsSync(tsPath) || existsSync(jsPath)) {
  viteConfigPath = existsSync(tsPath) ? tsPath : jsPath;
  const viteConfig = await readFile(viteConfigPath, 'utf-8');
  await writeFile(
    viteConfigPath,
    viteConfig.replace('defineConfig({', 'defineConfig({\n  run: { cacheScripts: true },'),
    'utf-8',
  );
} else {
  // Use .js to avoid TypeScript-ESLint "not found by the project service" errors
  // in projects whose tsconfig.json doesn't include vite.config.ts.
  viteConfigPath = jsPath;
  await writeFile(
    jsPath,
    `import { defineConfig } from 'vite-plus';\n\nexport default defineConfig({\n  run: { cacheScripts: true },\n});\n`,
    'utf-8',
  );
}
// Format the modified/created config to match project's style (avoids format check failures)
try {
  execSync(`npx prettier --write ${JSON.stringify(viteConfigPath)}`, { cwd, stdio: 'inherit' });
} catch {
  // prettier may not be installed; that's fine
}
