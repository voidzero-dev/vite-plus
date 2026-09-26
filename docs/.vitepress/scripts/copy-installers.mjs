import { execFileSync } from 'node:child_process';
import { copyFile, mkdir, readFile, writeFile } from 'node:fs/promises';

import { resolveDocsSiteOrigin } from '../site-origin.ts';

const siteOrigin = resolveDocsSiteOrigin();

// Piped installers must fetch legacy from the same production or preview deploy.
const origin = siteOrigin || 'https://viteplus.dev';
for (const name of ['install.sh', 'install.ps1', 'install-legacy.sh', 'install-legacy.ps1']) {
  const source = new URL(`../../../packages/cli/${name}`, import.meta.url);
  const destination = new URL(`../../public/${name}`, import.meta.url);
  const content = await readFile(source, 'utf8');
  await writeFile(
    destination,
    content.replace('https://viteplus.dev/install-legacy.', `${origin}/install-legacy.`),
  );
}

// Serve the exact skill from this checkout so production and preview setup
// instructions install the version that matches the deployed documentation.
const skillDestination = new URL('../../public/agent-setup/vite-plus/SKILL.md', import.meta.url);
await mkdir(new URL('.', skillDestination), { recursive: true });
await copyFile(new URL('../../../skills/vite-plus/SKILL.md', import.meta.url), skillDestination);

const promptTemplate = await readFile(
  new URL('../templates/agent-setup-prompt.md', import.meta.url),
  'utf8',
);
const placeholder = '__VITE_PLUS_SKILL_SOURCE__';
if (promptTemplate.split(placeholder).length !== 2) {
  throw new Error(`Expected exactly one ${placeholder} placeholder in the agent setup prompt`);
}

let skillSource = 'https://github.com/voidzero-dev/vite-plus/tree/main/skills/vite-plus';
if (siteOrigin) {
  const repository = process.env.DOCS_SKILL_REPOSITORY || 'voidzero-dev/vite-plus';
  if (!/^[\w.-]+\/[\w.-]+$/.test(repository)) {
    throw new Error(`Invalid DOCS_SKILL_REPOSITORY: ${repository}`);
  }
  const revision =
    process.env.DOCS_GIT_SHA ||
    process.env.WORKERS_CI_COMMIT_SHA ||
    execFileSync('git', ['rev-parse', 'HEAD'], {
      cwd: new URL('../../..', import.meta.url),
      encoding: 'utf8',
    }).trim();
  if (!/^[\da-f]{40}$/i.test(revision)) {
    throw new Error(`Invalid docs Git revision: ${revision}`);
  }
  skillSource = `https://github.com/${repository}/tree/${revision}/skills/vite-plus`;
}

const promptDestination = new URL('../../public/agent-setup/prompt.md', import.meta.url);
await mkdir(new URL('.', promptDestination), { recursive: true });
await writeFile(promptDestination, promptTemplate.replace(placeholder, skillSource));
